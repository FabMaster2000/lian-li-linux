use crate::app::AppState;
use crate::config::{xdg_config_home, xdg_runtime_dir};
use crate::errors::ApiResult;
use crate::models::{
    BackendAuthRuntime, BackendRuntime, ConfigDocument, DaemonRuntime, DaemonStatusResponse,
    DeviceCapability, DeviceControllerContext, DeviceHealthState, DevicePresentationUpdateRequest,
    DeviceState, DeviceView, DeviceWirelessContext, FanApplyRequest, FanApplyResponse,
    FanApplySkip, FanConfigDocument, FanCurveDocument, FanCurvePointDocument,
    FanCurveUpsertRequest, FanDeviceConfigDocument, FanManualRequest, FanSlotConfigDocument,
    FanSlotState, FanStateResponse, HealthResponse, LcdConfigDocument,
    LightingApplyDeviceState, LightingApplyRequest, LightingApplyResponse, LightingApplySkip,
    LightingBrightnessRequest, LightingColorRequest, LightingConfigDocument,
    LightingDeviceConfigDocument, LightingEffectRequest,
    LightingEffectRouteEntryDocument, LightingEffectRouteResponse,
    LightingEffectRouteSaveRequest, LightingFanLedZoneConfigDocument,
    LightingLedZoneConfigDocument, LightingStateResponse, LightingZoneConfigDocument,
    LightingZoneState,
    ProfileApplyResponse, ProfileApplySkip, ProfileDocument,
    ProfileFanDocument, ProfileLightingDocument, ProfileMetadataDocument, ProfileTargetsDocument,
    ProfileUpsertDocument, RuntimeResponse, SensorConfigDocument, SensorRangeDocument,
    SensorSourceDocument, VersionResponse, WirelessConnectResponse, WirelessDisconnectResponse,
    WirelessDiscoveryRefreshResponse,
};
use crate::storage::{DevicePresentationRecord, InventoryPresentation};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use futures_util::StreamExt;
use lianli_shared::config::{AppConfig, HidDriver, LcdConfig};
use lianli_shared::device_id::DeviceFamily;
use lianli_shared::fan::{FanConfig, FanCurve, FanGroup, FanSpeed, MB_SYNC_KEY};
use lianli_shared::ipc::{
    DeviceInfo, FanTemperaturePreview, IpcRequest, IpcResponse, TelemetrySnapshot,
    WirelessBindingState,
};
use lianli_shared::media::{MediaType, SensorDescriptor, SensorRange, SensorSourceConfig};
use lianli_shared::rgb::{
    RgbAppConfig, RgbDeviceConfig, RgbDirection, RgbEffect, RgbEffectRouteEntry,
    RgbFanLedZoneConfig, RgbLedZoneConfig, RgbMode, RgbScope, RgbZoneConfig,
    MAX_EFFECT_SPEED,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration as TokioDuration, MissedTickBehavior};
pub async fn health() -> ApiResult<Json<HealthResponse>> {
    Ok(Json(HealthResponse { status: "ok" }))
}

const METEOR_FIXED_BRIGHTNESS_PERCENT: u8 = 100;
const METEOR_FIXED_SMOOTHNESS_MS: u16 = 0;
const METEOR_FIXED_DIRECTION: RgbDirection = RgbDirection::Clockwise;

pub async fn version() -> ApiResult<Json<VersionResponse>> {
    Ok(Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
    }))
}

pub async fn websocket_events(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let events = state.events.clone();
    ws.on_upgrade(move |socket| handle_websocket(socket, events.subscribe()))
}

pub async fn runtime(State(state): State<AppState>) -> ApiResult<Json<RuntimeResponse>> {
    let backend = BackendRuntime {
        host: state.config.host.to_string(),
        port: state.config.port,
        log_level: state.config.log_level.clone(),
        config_path: state.config.backend_config_path.display().to_string(),
        profile_store_path: state.profiles.path().display().to_string(),
        auth: BackendAuthRuntime {
            enabled: state.config.auth.mode != crate::config::AuthMode::None,
            mode: state.config.auth.mode.as_str().to_string(),
            reload_requires_restart: true,
            basic_username_configured: state.config.auth.basic_username.is_some(),
            basic_password_configured: state.config.auth.basic_password.is_some(),
            token_configured: state.config.auth.bearer_token.is_some(),
            proxy_header: state.config.auth.proxy_header.clone(),
        },
    };

    let daemon = DaemonRuntime {
        socket_path: state.daemon.socket_path().display().to_string(),
        config_path: state.config.daemon_config_path.display().to_string(),
        xdg_runtime_dir: xdg_runtime_dir(),
        xdg_config_home: xdg_config_home(),
    };

    Ok(Json(RuntimeResponse { backend, daemon }))
}

pub async fn daemon_status(State(state): State<AppState>) -> ApiResult<Json<DaemonStatusResponse>> {
    let socket_path = state.daemon.socket_path().display().to_string();
    let result = state.daemon.send(&IpcRequest::Ping);

    let (reachable, error) = match result {
        Ok(IpcResponse::Ok { .. }) => (true, None),
        Ok(IpcResponse::Error { message }) => (false, Some(message)),
        Err(e) => (false, Some(e.to_string())),
    };

    Ok(Json(DaemonStatusResponse {
        reachable,
        socket_path,
        error,
    }))
}

pub async fn list_devices(State(state): State<AppState>) -> ApiResult<Json<Vec<DeviceView>>> {
    Ok(Json(load_device_views(&state)?))
}

pub async fn get_device(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<DeviceView>> {
    Ok(Json(find_device_view(&state, &id)?))
}

pub async fn update_device_presentation(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<DevicePresentationUpdateRequest>,
) -> ApiResult<Json<DeviceView>> {
    let device = require_device(&state, &id)?;
    let display_name = body.display_name.trim().to_string();
    if display_name.is_empty() {
        return Err(crate::errors::ApiError::BadRequest(
            "display_name must not be empty".to_string(),
        ));
    }

    let controller = controller_identity(&device);
    state.profiles.upsert_device_presentation(DevicePresentationRecord {
        device_id: id.clone(),
        display_name: Some(display_name.clone()),
        ui_order: Some(body.ui_order),
        physical_role: normalize_optional_value(body.physical_role),
        cluster_label: normalize_optional_value(body.cluster_label),
    })?;
    state
        .profiles
        .upsert_controller_label(&controller.id, normalize_optional_value(body.controller_label))?;

    state.events.publish(crate::events::WebEvent::new(
        "device.updated",
        "api",
        Some(id.clone()),
        serde_json::json!({
            "reason": "presentation_updated",
            "display_name": display_name,
            "ui_order": body.ui_order,
        }),
    ));

    Ok(Json(find_device_view(&state, &id)?))
}

pub async fn refresh_wireless_discovery(
    State(state): State<AppState>,
) -> ApiResult<Json<WirelessDiscoveryRefreshResponse>> {
    let response: WirelessDiscoveryRefreshResponse =
        unwrap_response(state.daemon.send(&IpcRequest::RefreshWirelessDiscovery)?)?;

    state.events.publish(crate::events::WebEvent::new(
        "device.updated",
        "api",
        None,
        serde_json::json!({
            "reason": "wireless_discovery_refreshed",
            "device_count": response.device_count,
        }),
    ));

    Ok(Json(response))
}

pub async fn connect_wireless_device(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<WirelessConnectResponse>> {
    let device = require_device(&state, &id)?;
    if device.wireless_channel.is_none() {
        return Err(crate::errors::ApiError::BadRequest(
            "device is not a wireless device".to_string(),
        ));
    }

    match device.wireless_binding_state {
        Some(WirelessBindingState::Connected) => {
            return Ok(Json(WirelessConnectResponse {
                device_id: id,
                connected: true,
            }));
        }
        Some(WirelessBindingState::Foreign) => {
            return Err(crate::errors::ApiError::BadRequest(
                "device is currently paired to another controller".to_string(),
            ));
        }
        _ => {}
    }

    let response: WirelessConnectResponse = unwrap_response(
        state
            .daemon
            .send(&IpcRequest::BindWirelessDevice {
                device_id: id.clone(),
            })?,
    )?;

    state.events.publish(crate::events::WebEvent::new(
        "device.updated",
        "api",
        Some(id),
        serde_json::json!({
            "reason": "wireless_connected",
        }),
    ));

    Ok(Json(response))
}
pub async fn disconnect_wireless_device(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<WirelessDisconnectResponse>> {
    let device = require_device(&state, &id)?;
    if device.wireless_channel.is_none() {
        return Err(crate::errors::ApiError::BadRequest(
            "device is not a wireless device".to_string(),
        ));
    }

    let response: WirelessDisconnectResponse = unwrap_response(
        state
            .daemon
            .send(&IpcRequest::UnbindWirelessDevice {
                device_id: id.clone(),
            })?,
    )?;

    state.events.publish(crate::events::WebEvent::new(
        "device.updated",
        "api",
        Some(id),
        serde_json::json!({
            "reason": "wireless_disconnected",
        }),
    ));

    Ok(Json(response))
}

pub async fn get_api_config(State(state): State<AppState>) -> ApiResult<Json<ConfigDocument>> {
    let cfg = get_config(&state)?;
    Ok(Json(config_document_from_app_config(cfg)))
}

pub async fn set_api_config(
    State(state): State<AppState>,
    Json(body): Json<ConfigDocument>,
) -> ApiResult<Json<ConfigDocument>> {
    let cfg = app_config_from_document(body, &state.config.daemon_config_path)?;
    save_app_config(&state, cfg.clone())?;
    state.events.publish_config_changed(
        "api",
        serde_json::json!({ "reason": "config_replaced" }),
    );
    Ok(Json(config_document_from_app_config(cfg)))
}

pub async fn list_profiles(State(state): State<AppState>) -> ApiResult<Json<Vec<ProfileDocument>>> {
    Ok(Json(state.profiles.list()?))
}

pub async fn create_profile(
    State(state): State<AppState>,
    Json(body): Json<ProfileUpsertDocument>,
) -> ApiResult<Json<ProfileDocument>> {
    let profile = profile_document_from_upsert(body, None)?;
    let stored = state.profiles.create(profile)?;
    Ok(Json(stored))
}

pub async fn update_profile(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<ProfileUpsertDocument>,
) -> ApiResult<Json<ProfileDocument>> {
    if body.id != id {
        return Err(crate::errors::ApiError::BadRequest(
            "profile id in path and body must match".to_string(),
        ));
    }

    let existing = state
        .profiles
        .get(&id)?
        .ok_or_else(|| crate::errors::ApiError::NotFound(format!("unknown profile id: {id}")))?;
    let profile = profile_document_from_upsert(body, Some(existing.metadata))?;
    let stored = state.profiles.update(&id, profile)?;
    Ok(Json(stored))
}

pub async fn delete_profile(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    state.profiles.delete(&id)?;
    Ok(Json(serde_json::json!({ "deleted": true, "id": id })))
}

pub async fn apply_profile(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<ProfileApplyResponse>> {
    let profile = state
        .profiles
        .get(&id)?
        .ok_or_else(|| crate::errors::ApiError::NotFound(format!("unknown profile id: {id}")))?;

    let devices: Vec<DeviceInfo> = unwrap_response(state.daemon.send(&IpcRequest::ListDevices)?)?;
    let target_devices = resolve_profile_targets(&profile.targets, &devices)?;
    let mut cfg = get_config(&state)?;
    let mut applied_lighting_device_ids = Vec::new();
    let mut applied_fan_device_ids = Vec::new();
    let mut skipped_devices = Vec::new();

    if let Some(lighting) = &profile.lighting {
        apply_profile_lighting(
            &mut cfg,
            &target_devices,
            lighting,
            &mut applied_lighting_device_ids,
            &mut skipped_devices,
        )?;
    }

    if let Some(fans) = &profile.fans {
        apply_profile_fans(
            &mut cfg,
            &target_devices,
            fans,
            &mut applied_fan_device_ids,
            &mut skipped_devices,
        )?;
    }

    if applied_lighting_device_ids.is_empty() && applied_fan_device_ids.is_empty() {
        return Err(crate::errors::ApiError::BadRequest(
            "profile did not match any compatible devices".to_string(),
        ));
    }

    let profile_id = profile.id.clone();
    let profile_name = profile.name.clone();
    save_app_config(&state, cfg)?;
    state.events.publish_config_changed(
        "api",
        serde_json::json!({
            "reason": "profile_applied",
            "profile_id": profile_id.clone(),
        }),
    );
    for device_id in &applied_lighting_device_ids {
        state.events.publish_lighting_changed(
            "api",
            device_id,
            serde_json::json!({
                "reason": "profile_apply",
                "profile_id": profile_id.clone(),
            }),
        );
    }
    for device_id in &applied_fan_device_ids {
        state.events.publish_fan_changed(
            "api",
            device_id,
            serde_json::json!({
                "reason": "profile_apply",
                "profile_id": profile_id.clone(),
            }),
        );
    }

    Ok(Json(ProfileApplyResponse {
        profile_id,
        profile_name,
        transaction_mode: "single_config_write".to_string(),
        rollback_supported: false,
        applied_lighting_device_ids,
        applied_fan_device_ids,
        skipped_devices,
    }))
}

pub async fn get_lighting_state(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<LightingStateResponse>> {
    let device = require_device(&state, &id)?;
    let cfg = get_config(&state)?;
    let rgb = cfg.rgb.unwrap_or_default();
    let zones = build_lighting_zones(&rgb, &device);
    Ok(Json(LightingStateResponse {
        device_id: id,
        zones,
    }))
}
pub async fn set_lighting_color(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<LightingColorRequest>,
) -> ApiResult<Json<LightingStateResponse>> {
    let device = require_device(&state, &id)?;
    let zone = body.zone.unwrap_or(0);
    let color = parse_color(&body.color)?;

    let cfg = get_config(&state)?;
    let mut rgb = cfg.rgb.unwrap_or_default();
    prune_lighting_zones(&mut rgb, &device);
    let mut effect = effect_for_zone(&rgb, &id, zone);
    effect.mode = RgbMode::Static;
    effect.colors = vec![color];
    upsert_zone_effect(&mut rgb, &id, zone, effect);

    save_rgb_config(&state, rgb.clone())?;
    state.events.publish_lighting_changed(
        "api",
        &id,
        serde_json::json!({
            "reason": "color_set",
            "zone": zone,
            "color": rgb_to_hex(color),
        }),
    );
    let zones = build_lighting_zones(&rgb, &device);
    Ok(Json(LightingStateResponse {
        device_id: id,
        zones,
    }))
}
pub async fn set_lighting_effect(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<LightingEffectRequest>,
) -> ApiResult<Json<LightingStateResponse>> {
    let device = require_device(&state, &id)?;
    let zone = body.zone.unwrap_or(0);
    let mode = parse_rgb_mode(&body.effect)?;

    let cfg = get_config(&state)?;
    let mut rgb = cfg.rgb.unwrap_or_default();
    prune_lighting_zones(&mut rgb, &device);
    let mut effect = effect_for_zone(&rgb, &id, zone);
    effect.mode = mode;

    if let Some(speed) = body.speed {
        validate_lighting_speed(speed)?;
        effect.speed = speed;
    }

    if mode == RgbMode::Meteor {
        effect.brightness = percent_to_brightness(METEOR_FIXED_BRIGHTNESS_PERCENT);
        effect.smoothness_ms = METEOR_FIXED_SMOOTHNESS_MS;
        effect.direction = METEOR_FIXED_DIRECTION;
    } else if let Some(brightness) = body.brightness {
        validate_lighting_brightness(brightness)?;
        effect.brightness = percent_to_brightness(brightness);
    }

    let palette = parse_palette(&body.colors, body.color.as_ref())?;
    if !palette.is_empty() {
        effect.colors = palette;
    }

    if mode != RgbMode::Meteor {
        if let Some(direction) = body.direction {
            effect.direction = parse_direction(&direction)?;
        }
    }

    if let Some(scope) = body.scope {
        effect.scope = parse_scope(&scope)?;
    }

    if mode != RgbMode::Meteor {
        if let Some(smoothness_ms) = body.smoothness_ms {
            validate_lighting_smoothness(smoothness_ms)?;
            effect.smoothness_ms = smoothness_ms;
        }
    }

    upsert_zone_effect(&mut rgb, &id, zone, effect);
    save_rgb_config(&state, rgb.clone())?;
    state.events.publish_lighting_changed(
        "api",
        &id,
        serde_json::json!({
            "reason": "effect_set",
            "zone": zone,
            "effect": rgb_mode_name(mode),
        }),
    );
    let zones = build_lighting_zones(&rgb, &device);
    Ok(Json(LightingStateResponse {
        device_id: id,
        zones,
    }))
}
pub async fn apply_lighting_workbench(
    State(state): State<AppState>,
    Json(body): Json<LightingApplyRequest>,
) -> ApiResult<Json<LightingApplyResponse>> {
    let target_mode = parse_lighting_target_mode(&body.target_mode)?;
    let zone_mode = parse_lighting_zone_mode(&body.zone_mode)?;
    let mode = parse_rgb_mode(&body.effect)?;
    validate_lighting_brightness(body.brightness)?;
    if let Some(speed) = body.speed {
        validate_lighting_speed(speed)?;
    }
    if mode != RgbMode::Meteor && mode != RgbMode::Runway {
        if let Some(smoothness_ms) = body.smoothness_ms {
            validate_lighting_smoothness(smoothness_ms)?;
        }
    }
    let palette = parse_palette(&body.colors, body.color.as_ref())?;

    let cfg = get_config(&state)?;
    let mut rgb = cfg.rgb.unwrap_or_default();
    let devices: Vec<DeviceInfo> = unwrap_response(state.daemon.send(&IpcRequest::ListDevices)?)?;
    let requested_device_ids = resolve_lighting_targets(target_mode, &body, &devices, &rgb)?;
    let device_map = devices
        .into_iter()
        .map(|device| (device.device_id.clone(), device))
        .collect::<HashMap<_, _>>();
    let mut applied_devices = Vec::new();
    let mut skipped_devices = Vec::new();

    for device_id in &requested_device_ids {
        let Some(device) = device_map.get(device_id) else {
            skipped_devices.push(LightingApplySkip {
                device_id: device_id.clone(),
                reason: "device is not present in the current inventory".to_string(),
            });
            continue;
        };

        if let Some(reason) = lighting_apply_skip_reason(device, mode) {
            skipped_devices.push(LightingApplySkip {
                device_id: device.device_id.clone(),
                reason,
            });
            continue;
        }

        prune_lighting_zones(&mut rgb, device);
        let zones = match zones_for_lighting_apply(device, &rgb, zone_mode, body.zone) {
            Ok(zones) => zones,
            Err(reason) => {
                skipped_devices.push(LightingApplySkip {
                    device_id: device.device_id.clone(),
                    reason,
                });
                continue;
            }
        };

        for zone in zones {
            let mut effect = effect_for_zone(&rgb, &device.device_id, zone);
            effect.mode = mode;
            if !palette.is_empty() {
                effect.colors = palette.clone();
            }
            if mode == RgbMode::Meteor || mode == RgbMode::Runway {
                effect.brightness = percent_to_brightness(METEOR_FIXED_BRIGHTNESS_PERCENT);
                effect.smoothness_ms = METEOR_FIXED_SMOOTHNESS_MS;
                effect.direction = METEOR_FIXED_DIRECTION;
            } else {
                effect.brightness = percent_to_brightness(body.brightness);
            }
            if let Some(speed) = body.speed {
                effect.speed = speed;
            }
            if mode != RgbMode::Meteor && mode != RgbMode::Runway {
                if let Some(direction) = &body.direction {
                    effect.direction = parse_direction(direction)?;
                }
            }
            if let Some(scope) = &body.scope {
                effect.scope = parse_scope(scope)?;
            }
            if mode != RgbMode::Meteor && mode != RgbMode::Runway {
                if let Some(smoothness_ms) = body.smoothness_ms {
                    effect.smoothness_ms = smoothness_ms;
                }
            }
            upsert_zone_effect(&mut rgb, &device.device_id, zone, effect);
        }

        applied_devices.push(LightingApplyDeviceState {
            device_id: device.device_id.clone(),
            zones: build_lighting_zones(&rgb, device),
        });
    }

    if !applied_devices.is_empty() {
        save_rgb_config(&state, rgb)?;
        for device in &applied_devices {
            state.events.publish_lighting_changed(
                "api",
                &device.device_id,
                serde_json::json!({
                    "reason": "workbench_apply",
                    "target_mode": body.target_mode,
                    "zone_mode": body.zone_mode,
                    "sync_selected": body.sync_selected,
                }),
            );
        }
    }

    Ok(Json(LightingApplyResponse {
        target_mode: body.target_mode,
        zone_mode: body.zone_mode,
        requested_device_ids,
        applied_devices,
        skipped_devices,
        sync_selected: body.sync_selected,
    }))
}

pub async fn set_lighting_brightness(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<LightingBrightnessRequest>,
) -> ApiResult<Json<LightingStateResponse>> {
    let device = require_device(&state, &id)?;
    let zone = body.zone.unwrap_or(0);
    if body.percent > 100 {
        return Err(crate::errors::ApiError::BadRequest(
            "brightness percent must be 0-100".to_string(),
        ));
    }

    let cfg = get_config(&state)?;
    let mut rgb = cfg.rgb.unwrap_or_default();
    prune_lighting_zones(&mut rgb, &device);
    let mut effect = effect_for_zone(&rgb, &id, zone);
    effect.brightness = percent_to_brightness(body.percent);
    upsert_zone_effect(&mut rgb, &id, zone, effect);
    save_rgb_config(&state, rgb.clone())?;
    state.events.publish_lighting_changed(
        "api",
        &id,
        serde_json::json!({
            "reason": "brightness_set",
            "zone": zone,
            "brightness_percent": body.percent,
        }),
    );

    let zones = build_lighting_zones(&rgb, &device);
    Ok(Json(LightingStateResponse {
        device_id: id,
        zones,
    }))
}

pub async fn get_lighting_effect_route(
    State(state): State<AppState>,
) -> ApiResult<Json<LightingEffectRouteResponse>> {
    let rgb = get_config(&state)?.rgb.unwrap_or_default();
    Ok(Json(LightingEffectRouteResponse {
        route: document_effect_route_from_config(&rgb.effect_route),
    }))
}

pub async fn save_lighting_effect_route(
    State(state): State<AppState>,
    Json(body): Json<LightingEffectRouteSaveRequest>,
) -> ApiResult<Json<LightingEffectRouteResponse>> {
    let mut rgb = get_config(&state)?.rgb.unwrap_or_default();
    rgb.effect_route = effect_route_from_documents(body.route)?;
    let route = document_effect_route_from_config(&rgb.effect_route);

    save_rgb_config(&state, rgb)?;
    state.events.publish_config_changed(
        "api",
        serde_json::json!({
            "reason": "lighting_effect_route_saved",
            "route_size": route.len(),
        }),
    );

    Ok(Json(LightingEffectRouteResponse { route }))
}

pub async fn get_fan_state(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<FanStateResponse>> {
    let device = require_device(&state, &id)?;
    let cfg = get_config(&state)?;
    let telemetry: TelemetrySnapshot =
        unwrap_response(state.daemon.send(&IpcRequest::GetTelemetry)?)?;

    Ok(Json(build_fan_state_response(&cfg, &telemetry, &device)))
}

pub async fn set_fan_manual(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<FanManualRequest>,
) -> ApiResult<Json<FanStateResponse>> {
    let device = require_device(&state, &id)?;
    if body.percent > 100 {
        return Err(crate::errors::ApiError::BadRequest(
            "fan percent must be 0-100".to_string(),
        ));
    }
    if let Some(slot) = body.slot {
        if !(1..=4).contains(&slot) {
            return Err(crate::errors::ApiError::BadRequest(
                "slot must be 1-4".to_string(),
            ));
        }
    }

    let cfg = get_config(&state)?;
    let mut fan_cfg = cfg.fans.clone().unwrap_or_else(default_fan_config);
    let effective_percent = normalize_manual_percent_for_device(&device, body.percent);
    let pwm = percent_to_pwm(effective_percent);
    let group = ensure_fan_group_mut(&mut fan_cfg, &id);

    if let Some(slot) = body.slot {
        let idx = (slot - 1) as usize;
        let mut speeds = group.speeds.clone();
        speeds[idx] = FanSpeed::Constant(pwm);
        group.speeds = speeds;
    } else {
        group.speeds = default_fan_speeds(pwm);
    }

    save_fan_config(&state, fan_cfg.clone())?;
    state.events.publish_fan_changed(
        "api",
        &id,
        serde_json::json!({
            "reason": "manual_set",
            "slot": body.slot,
            "percent": effective_percent,
        }),
    );

    let next_cfg = cfg.with_fans(fan_cfg);
    let telemetry: TelemetrySnapshot =
        unwrap_response(state.daemon.send(&IpcRequest::GetTelemetry)?)?;

    Ok(Json(build_fan_state_response(&next_cfg, &telemetry, &device)))
}

#[derive(Debug, serde::Deserialize)]
pub struct FanTemperaturePreviewQuery {
    source: String,
}

pub async fn preview_fan_temperature(
    State(state): State<AppState>,
    Query(query): Query<FanTemperaturePreviewQuery>,
) -> ApiResult<Json<FanTemperaturePreview>> {
    let source = query.source.trim();
    if source.is_empty() {
        return Err(crate::errors::ApiError::BadRequest(
            "temperature source must not be empty".to_string(),
        ));
    }

    let preview: FanTemperaturePreview = unwrap_response(
        state.daemon.send(&IpcRequest::PreviewFanTemperature {
            source: source.to_string(),
        })?,
    )?;

    Ok(Json(preview))
}

pub async fn list_fan_curves(State(state): State<AppState>) -> ApiResult<Json<Vec<FanCurveDocument>>> {
    let cfg = get_config(&state)?;
    Ok(Json(fan_document_from_app_config(&cfg).curves))
}

pub async fn create_fan_curve(
    State(state): State<AppState>,
    Json(body): Json<FanCurveUpsertRequest>,
) -> ApiResult<Json<FanCurveDocument>> {
    let mut cfg = get_config(&state)?;
    let mut fan_doc = fan_document_from_app_config(&cfg);
    let saved_curve = FanCurveDocument {
        name: body.name,
        temperature_source: body.temperature_source,
        points: body.points,
    };
    fan_doc.curves.push(saved_curve.clone());
    cfg.fan_curves = fan_curves_from_document(&fan_doc)?;
    save_app_config(&state, cfg)?;
    state.events.publish_config_changed(
        "api",
        serde_json::json!({
            "reason": "fan_curve_created",
            "curve": saved_curve.name,
        }),
    );
    Ok(Json(saved_curve))
}

pub async fn update_fan_curve(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<FanCurveUpsertRequest>,
) -> ApiResult<Json<FanCurveDocument>> {
    let mut cfg = get_config(&state)?;
    let mut fan_doc = fan_document_from_app_config(&cfg);
    let mut replaced = false;
    let saved_curve = FanCurveDocument {
        name: body.name,
        temperature_source: body.temperature_source,
        points: body.points,
    };

    for curve in &mut fan_doc.curves {
        if curve.name == name {
            *curve = saved_curve.clone();
            replaced = true;
            break;
        }
    }

    if !replaced {
        return Err(crate::errors::ApiError::NotFound(format!(
            "unknown fan curve: {name}"
        )));
    }

    cfg.fan_curves = fan_curves_from_document(&fan_doc)?;
    if name != saved_curve.name {
        let curve_names = cfg
            .fan_curves
            .iter()
            .map(|curve| curve.name.clone())
            .collect::<HashSet<_>>();
        let fans_doc = fan_document_from_app_config(&cfg);
        cfg.fans = fan_config_from_document(fans_doc, &curve_names)?;
    }
    save_app_config(&state, cfg)?;
    state.events.publish_config_changed(
        "api",
        serde_json::json!({
            "reason": "fan_curve_updated",
            "curve": saved_curve.name,
            "previous_name": name,
        }),
    );
    Ok(Json(saved_curve))
}

pub async fn delete_fan_curve(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut cfg = get_config(&state)?;
    let in_use = cfg
        .fans
        .as_ref()
        .map(|fans| {
            fans.speeds
                .iter()
                .flat_map(|group| group.speeds.iter())
                .filter(|speed| matches!(speed, FanSpeed::Curve(curve_name) if curve_name == &name))
                .count()
        })
        .unwrap_or(0);

    if in_use > 0 {
        return Err(crate::errors::ApiError::BadRequest(format!(
            "fan curve '{name}' is still referenced by {in_use} slot(s)"
        )));
    }

    let original_len = cfg.fan_curves.len();
    cfg.fan_curves.retain(|curve| curve.name != name);
    if cfg.fan_curves.len() == original_len {
        return Err(crate::errors::ApiError::NotFound(format!(
            "unknown fan curve: {name}"
        )));
    }

    save_app_config(&state, cfg)?;
    state.events.publish_config_changed(
        "api",
        serde_json::json!({
            "reason": "fan_curve_deleted",
            "curve": name,
        }),
    );

    Ok(Json(serde_json::json!({ "deleted": true, "name": name })))
}

pub async fn apply_fan_workbench(
    State(state): State<AppState>,
    Json(body): Json<FanApplyRequest>,
) -> ApiResult<Json<FanApplyResponse>> {
    let target_mode = parse_fan_target_mode(&body.target_mode)?;
    let workbench_mode = parse_fan_workbench_mode(&body.mode)?;
    let devices: Vec<DeviceInfo> = unwrap_response(state.daemon.send(&IpcRequest::ListDevices)?)?;
    let requested_device_ids = resolve_fan_targets(target_mode, &body, &devices)?;
    let device_map = devices
        .into_iter()
        .map(|device| (device.device_id.clone(), device))
        .collect::<HashMap<_, _>>();
    let cfg = get_config(&state)?;
    let mut fan_cfg = cfg.fans.clone().unwrap_or_else(default_fan_config);
    let curve_names = cfg
        .fan_curves
        .iter()
        .map(|curve| curve.name.clone())
        .collect::<HashSet<_>>();
    let mut applied_device_ids = Vec::new();
    let mut skipped_devices = Vec::new();
    let manual_percent = if matches!(workbench_mode, FanWorkbenchMode::Manual) {
        let percent = body.percent.ok_or_else(|| {
            crate::errors::ApiError::BadRequest(
                "percent is required for manual fan mode".to_string(),
            )
        })?;
        if percent > 100 {
            return Err(crate::errors::ApiError::BadRequest(
                "fan percent must be 0-100".to_string(),
            ));
        }
        Some(percent)
    } else {
        None
    };
    let curve_name = if matches!(workbench_mode, FanWorkbenchMode::Curve) {
        let curve_name = body.curve.clone().ok_or_else(|| {
            crate::errors::ApiError::BadRequest(
                "curve is required for curve fan mode".to_string(),
            )
        })?;
        if !curve_names.contains(&curve_name) {
            return Err(crate::errors::ApiError::BadRequest(format!(
                "unknown fan curve: {curve_name}"
            )));
        }
        Some(curve_name)
    } else {
        None
    };

    for device_id in &requested_device_ids {
        let Some(device) = device_map.get(device_id) else {
            skipped_devices.push(FanApplySkip {
                device_id: device_id.clone(),
                reason: "device is not present in the current inventory".to_string(),
            });
            continue;
        };

        if !device.has_fan {
            skipped_devices.push(FanApplySkip {
                device_id: device.device_id.clone(),
                reason: "device has no fan capability".to_string(),
            });
            continue;
        }

        if matches!(workbench_mode, FanWorkbenchMode::MbSync) && !device.mb_sync_support {
            skipped_devices.push(FanApplySkip {
                device_id: device.device_id.clone(),
                reason: "motherboard RPM sync is not supported on this device".to_string(),
            });
            continue;
        }

        if matches!(workbench_mode, FanWorkbenchMode::StartStop) {
            skipped_devices.push(FanApplySkip {
                device_id: device.device_id.clone(),
                reason: "start/stop mode is not supported by the current backend model".to_string(),
            });
            continue;
        }

        let speed = match workbench_mode {
            FanWorkbenchMode::Manual => {
                let percent = manual_percent.expect("manual percent validated above");
                let effective_percent = normalize_manual_percent_for_device(device, percent);
                FanSpeed::Constant(percent_to_pwm(effective_percent))
            }
            FanWorkbenchMode::Curve => {
                FanSpeed::Curve(curve_name.clone().expect("curve validated above"))
            }
            FanWorkbenchMode::MbSync => FanSpeed::Curve(MB_SYNC_KEY.to_string()),
            FanWorkbenchMode::StartStop => {
                skipped_devices.push(FanApplySkip {
                    device_id: device.device_id.clone(),
                    reason: "fan mode could not be applied".to_string(),
                });
                continue;
            }
        };

        let group = ensure_fan_group_mut(&mut fan_cfg, &device.device_id);
        group.speeds = std::array::from_fn(|_| speed.clone());
        applied_device_ids.push(device.device_id.clone());
    }

    if !applied_device_ids.is_empty() {
        save_fan_config(&state, fan_cfg.clone())?;
        for device_id in &applied_device_ids {
            state.events.publish_fan_changed(
                "api",
                device_id,
                serde_json::json!({
                    "reason": "workbench_apply",
                    "target_mode": body.target_mode,
                    "mode": body.mode,
                    "curve": body.curve,
                    "percent": body.percent,
                }),
            );
        }
    }

    let telemetry: TelemetrySnapshot =
        unwrap_response(state.daemon.send(&IpcRequest::GetTelemetry)?)?;
    let next_cfg = cfg.with_fans(fan_cfg);
    let applied_devices = applied_device_ids
        .iter()
        .filter_map(|device_id| device_map.get(device_id))
        .map(|device| build_fan_state_response(&next_cfg, &telemetry, device))
        .collect();

    Ok(Json(FanApplyResponse {
        target_mode: body.target_mode,
        mode: body.mode,
        requested_device_ids,
        applied_devices,
        skipped_devices,
    }))
}

async fn handle_websocket(
    mut socket: WebSocket,
    mut events: broadcast::Receiver<crate::events::WebEvent>,
) {
    let mut heartbeat = interval(TokioDuration::from_secs(15));
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            event = events.recv() => {
                match event {
                    Ok(event) => {
                        let payload = match serde_json::to_string(&event) {
                            Ok(payload) => payload,
                            Err(err) => {
                                tracing::warn!("failed to serialize websocket event: {err}");
                                continue;
                            }
                        };

                        if socket.send(Message::Text(payload)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!("websocket client lagged behind by {skipped} events");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = socket.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Text(_)))
                    | Some(Ok(Message::Binary(_)))
                    | Some(Ok(Message::Pong(_))) => {}
                    Some(Err(err)) => {
                        tracing::debug!("websocket receive error: {err}");
                        break;
                    }
                }
            }
            _ = heartbeat.tick() => {
                if socket.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
            }
        }
    }
}

#[derive(Clone)]
struct ControllerIdentity {
    id: String,
    kind: String,
    default_label: String,
}

#[derive(Clone, Copy)]
enum LightingTargetMode {
    Single,
    Selected,
    All,
    Route,
}

#[derive(Clone, Copy)]
enum LightingZoneMode {
    Active,
    AllZones,
}

#[derive(Clone, Copy)]
enum FanTargetMode {
    Single,
    Selected,
    All,
}

#[derive(Clone, Copy)]
enum FanWorkbenchMode {
    Manual,
    Curve,
    MbSync,
    StartStop,
}

fn parse_lighting_target_mode(input: &str) -> ApiResult<LightingTargetMode> {
    match input.trim().to_lowercase().as_str() {
        "single" => Ok(LightingTargetMode::Single),
        "selected" => Ok(LightingTargetMode::Selected),
        "all" => Ok(LightingTargetMode::All),
        "route" => Ok(LightingTargetMode::Route),
        other => Err(crate::errors::ApiError::BadRequest(format!(
            "unknown lighting target mode: {other}"
        ))),
    }
}

fn parse_lighting_zone_mode(input: &str) -> ApiResult<LightingZoneMode> {
    match input.trim().to_lowercase().as_str() {
        "active" => Ok(LightingZoneMode::Active),
        "all_zones" | "all-zones" | "allzones" => Ok(LightingZoneMode::AllZones),
        other => Err(crate::errors::ApiError::BadRequest(format!(
            "unknown lighting zone mode: {other}"
        ))),
    }
}

fn resolve_lighting_targets(
    target_mode: LightingTargetMode,
    body: &LightingApplyRequest,
    devices: &[DeviceInfo],
    rgb: &RgbAppConfig,
) -> ApiResult<Vec<String>> {
    match target_mode {
        LightingTargetMode::Single => {
            let device_id = body
                .device_id
                .clone()
                .or_else(|| body.device_ids.first().cloned())
                .ok_or_else(|| {
                    crate::errors::ApiError::BadRequest(
                        "device_id required for single lighting target mode".to_string(),
                    )
                })?;
            Ok(vec![device_id])
        }
        LightingTargetMode::Selected => {
            let mut device_ids = body
                .device_ids
                .iter()
                .filter(|device_id| !device_id.trim().is_empty())
                .cloned()
                .collect::<Vec<_>>();
            device_ids.sort();
            device_ids.dedup();
            if device_ids.is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "device_ids required for selected lighting target mode".to_string(),
                ));
            }
            Ok(device_ids)
        }
        LightingTargetMode::All => Ok(devices
            .iter()
            .filter(|device| device.has_rgb)
            .map(|device| device.device_id.clone())
            .collect()),
        LightingTargetMode::Route => {
            let mut seen_device_ids = HashSet::new();
            let route_device_ids = rgb
                .effect_route
                .iter()
                .filter(|entry| !entry.device_id.trim().is_empty())
                .filter(|entry| seen_device_ids.insert(entry.device_id.clone()))
                .map(|entry| entry.device_id.clone())
                .collect::<Vec<_>>();

            if route_device_ids.is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "saved lighting effect route is empty".to_string(),
                ));
            }

            Ok(route_device_ids)
        }
    }
}

fn parse_fan_target_mode(input: &str) -> ApiResult<FanTargetMode> {
    match input.trim().to_lowercase().as_str() {
        "single" => Ok(FanTargetMode::Single),
        "selected" => Ok(FanTargetMode::Selected),
        "all" => Ok(FanTargetMode::All),
        other => Err(crate::errors::ApiError::BadRequest(format!(
            "unknown fan target mode: {other}"
        ))),
    }
}

fn parse_fan_workbench_mode(input: &str) -> ApiResult<FanWorkbenchMode> {
    match input.trim().to_lowercase().as_str() {
        "manual" => Ok(FanWorkbenchMode::Manual),
        "curve" => Ok(FanWorkbenchMode::Curve),
        "mb_sync" | "mb-sync" | "mbsync" => Ok(FanWorkbenchMode::MbSync),
        "start_stop" | "start-stop" | "startstop" => Ok(FanWorkbenchMode::StartStop),
        other => Err(crate::errors::ApiError::BadRequest(format!(
            "unknown fan mode: {other}"
        ))),
    }
}

fn resolve_fan_targets(
    target_mode: FanTargetMode,
    body: &FanApplyRequest,
    devices: &[DeviceInfo],
) -> ApiResult<Vec<String>> {
    match target_mode {
        FanTargetMode::Single => {
            let device_id = body
                .device_id
                .clone()
                .or_else(|| body.device_ids.first().cloned())
                .ok_or_else(|| {
                    crate::errors::ApiError::BadRequest(
                        "device_id required for single fan target mode".to_string(),
                    )
                })?;
            Ok(vec![device_id])
        }
        FanTargetMode::Selected => {
            let mut device_ids = body
                .device_ids
                .iter()
                .filter(|device_id| !device_id.trim().is_empty())
                .cloned()
                .collect::<Vec<_>>();
            device_ids.sort();
            device_ids.dedup();
            if device_ids.is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "device_ids required for selected fan target mode".to_string(),
                ));
            }
            Ok(device_ids)
        }
        FanTargetMode::All => Ok(devices
            .iter()
            .filter(|device| device.has_fan)
            .map(|device| device.device_id.clone())
            .collect()),
    }
}

fn validate_lighting_speed(speed: u8) -> ApiResult<()> {
    if speed > MAX_EFFECT_SPEED {
        return Err(crate::errors::ApiError::BadRequest(
            format!("speed must be 0-{MAX_EFFECT_SPEED}"),
        ));
    }
    Ok(())
}

fn validate_lighting_brightness(brightness: u8) -> ApiResult<()> {
    if brightness > 100 {
        return Err(crate::errors::ApiError::BadRequest(
            "brightness percent must be 0-100".to_string(),
        ));
    }
    Ok(())
}

const MAX_SMOOTHNESS_MS: u16 = 3000;

fn validate_lighting_smoothness(smoothness_ms: u16) -> ApiResult<()> {
    if smoothness_ms > MAX_SMOOTHNESS_MS {
        return Err(crate::errors::ApiError::BadRequest(
            format!("smoothness_ms must be 0-{MAX_SMOOTHNESS_MS}"),
        ));
    }
    Ok(())
}

fn parse_palette(colors: &[crate::models::ColorValue], color: Option<&crate::models::ColorValue>) -> ApiResult<Vec<[u8; 3]>> {
    let mut palette = if !colors.is_empty() {
        colors
            .iter()
            .map(parse_color)
            .collect::<ApiResult<Vec<_>>>()?
    } else if let Some(color) = color {
        vec![parse_color(color)?]
    } else {
        vec![[255, 255, 255]]
    };

    if palette.len() > 4 {
        return Err(crate::errors::ApiError::BadRequest(
            "palette supports up to 4 colors".to_string(),
        ));
    }

    if palette.is_empty() {
        palette.push([255, 255, 255]);
    }

    Ok(palette)
}

fn lighting_apply_skip_reason(device: &DeviceInfo, mode: RgbMode) -> Option<String> {
    if !device.has_rgb {
        return Some("device does not expose RGB control".to_string());
    }

    if mode == RgbMode::Direct {
        return Some("Direct mode is not exposed by the current web workbench".to_string());
    }

    if matches!(mode, RgbMode::BigBang | RgbMode::Vortex | RgbMode::Pump | RgbMode::ColorsMorph)
        && !device.has_pump
    {
        return Some("selected effect is limited to pump-capable devices".to_string());
    }

    None
}

fn zones_for_lighting_apply(
    device: &DeviceInfo,
    rgb: &RgbAppConfig,
    zone_mode: LightingZoneMode,
    zone: Option<u8>,
) -> Result<Vec<u8>, String> {
    let available_zones = effective_zone_indexes(rgb, device);

    match zone_mode {
        LightingZoneMode::Active => {
            let requested_zone = zone.unwrap_or(0);
            if !available_zones.contains(&requested_zone) {
                return Err(format!(
                    "requested zone {requested_zone} is outside the available zone range"
                ));
            }
            Ok(vec![requested_zone])
        }
        LightingZoneMode::AllZones => {
            if available_zones.is_empty() {
                return Err("device reports no RGB zones".to_string());
            }
            Ok(available_zones)
        }
    }
}

fn load_device_views(state: &AppState) -> ApiResult<Vec<DeviceView>> {
    let devices: Vec<DeviceInfo> = unwrap_response(state.daemon.send(&IpcRequest::ListDevices)?)?;
    let telemetry: TelemetrySnapshot =
        unwrap_response(state.daemon.send(&IpcRequest::GetTelemetry)?)?;
    let presentation = state.profiles.inventory_presentation()?;
    let controller_counts = devices.iter().fold(HashMap::new(), |mut counts, device| {
        let controller = controller_identity(device);
        *counts.entry(controller.id).or_insert(0) += 1;
        counts
    });
    let mut views = devices
        .into_iter()
        .enumerate()
        .map(|(index, device)| device_view(device, index, &telemetry, &presentation, &controller_counts))
        .collect::<Vec<_>>();
    views.sort_by(|left, right| {
        left.ui_order
            .cmp(&right.ui_order)
            .then(left.controller.label.cmp(&right.controller.label))
            .then(left.display_name.cmp(&right.display_name))
            .then(left.id.cmp(&right.id))
    });
    Ok(views)
}

fn find_device_view(state: &AppState, id: &str) -> ApiResult<DeviceView> {
    load_device_views(state)?
        .into_iter()
        .find(|device| device.id == id)
        .ok_or_else(|| crate::errors::ApiError::NotFound(format!("unknown device id: {id}")))
}

fn device_view(
    device: DeviceInfo,
    index: usize,
    telemetry: &TelemetrySnapshot,
    presentation: &InventoryPresentation,
    controller_counts: &HashMap<String, usize>,
) -> DeviceView {
    let telemetry_fan_rpms = telemetry.fan_rpms.get(&device.device_id).cloned();
    let online = device_online(&device, &telemetry_fan_rpms);
    let fan_rpms = online.then(|| telemetry_fan_rpms.clone()).flatten();
    let coolant_temp = telemetry.coolant_temps.get(&device.device_id).cloned();
    let presentation_record = presentation.device_presentations.get(&device.device_id);
    let controller = controller_identity(&device);
    let controller_label = presentation
        .controller_labels
        .get(&controller.id)
        .cloned()
        .flatten()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| controller.default_label.clone());
    let display_name = presentation_record
        .and_then(|record| record.display_name.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_device_display_name(&device));
    let cluster_label = presentation_record
        .and_then(|record| record.cluster_label.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| display_name.clone());
    let ui_order = presentation_record
        .and_then(|record| record.ui_order)
        .unwrap_or((index as u32) * 10);
    let physical_role = presentation_record
        .and_then(|record| record.physical_role.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_physical_role(&device));
    let controller_device_count = controller_counts.get(&controller.id).copied().unwrap_or(1);
    let capability_summary = capability_summary(&device);
    let current_mode_summary =
        current_mode_summary(&device, online, &fan_rpms, telemetry_fan_rpms.as_ref(), telemetry);
    let health =
        health_state(&device, online, &fan_rpms, telemetry_fan_rpms.as_ref(), coolant_temp);

    DeviceView {
        id: device.device_id.clone(),
        name: device.name,
        display_name,
        family: format!("{:?}", device.family),
        online,
        ui_order,
        physical_role,
        capability_summary,
        current_mode_summary,
        controller: DeviceControllerContext {
            id: controller.id.clone(),
            label: if controller.kind == "wireless_mesh" || controller_device_count <= 1 {
                controller_label
            } else {
                format!("{} ({controller_device_count} devices)", controller_label)
            },
            kind: controller.kind,
        },
        wireless: if device.device_id.starts_with("wireless:") {
            Some(DeviceWirelessContext {
                transport: "wireless".to_string(),
                channel: device.wireless_channel,
                group_id: Some(device.device_id.clone()),
                group_label: Some(cluster_label),
                binding_state: device
                    .wireless_binding_state
                    .map(wireless_binding_state_name),
                master_mac: device.wireless_master_mac.clone(),
            })
        } else {
            None
        },
        health,
        capabilities: DeviceCapability {
            has_fan: device.has_fan,
            has_rgb: device.has_rgb,
            has_lcd: device.has_lcd,
            has_pump: device.has_pump,
            fan_count: device.fan_count,
            per_fan_control: device.per_fan_control,
            mb_sync_support: device.mb_sync_support,
            rgb_zone_count: device.rgb_zone_count,
        },
        state: DeviceState {
            fan_rpms,
            coolant_temp,
            streaming_active: Some(telemetry.streaming_active),
        },
    }
}

fn wireless_binding_state_name(state: WirelessBindingState) -> String {
    match state {
        WirelessBindingState::Connected => "connected".to_string(),
        WirelessBindingState::Available => "available".to_string(),
        WirelessBindingState::Foreign => "foreign".to_string(),
    }
}
fn controller_identity(device: &DeviceInfo) -> ControllerIdentity {
    if device.device_id.starts_with("wireless:") {
        return ControllerIdentity {
            id: "wireless:mesh".to_string(),
            kind: "wireless_mesh".to_string(),
            default_label: "Wireless dongle".to_string(),
        };
    }

    if let Some((base_id, _)) = device.device_id.rsplit_once(":port") {
        return ControllerIdentity {
            id: base_id.to_string(),
            kind: "wired_controller".to_string(),
            default_label: device
                .name
                .split(" Port ")
                .next()
                .unwrap_or(&device.name)
                .to_string(),
        };
    }

    ControllerIdentity {
        id: device.device_id.clone(),
        kind: if device.has_fan || device.has_rgb {
            "controller".to_string()
        } else {
            "standalone".to_string()
        },
        default_label: device.name.clone(),
    }
}

fn default_device_display_name(device: &DeviceInfo) -> String {
    if device.device_id.starts_with("wireless:") {
        return format!(
            "{} {} [{}]",
            device.name,
            wireless_cluster_role(device),
            short_wireless_identity(&device.device_id)
        );
    }

    device.name.clone()
}

fn wireless_cluster_role(device: &DeviceInfo) -> String {
    match device.fan_count {
        Some(1) => "single-fan cluster".to_string(),
        Some(count) => format!("{count}-fan cluster"),
        None => "wireless cluster".to_string(),
    }
}

fn short_wireless_identity(device_id: &str) -> String {
    let tail = device_id.strip_prefix("wireless:").unwrap_or(device_id);
    let segments = tail.split(':').collect::<Vec<_>>();

    if segments.len() >= 4 {
        return format!(
            "{}:{}..{}:{}",
            segments[0],
            segments[1],
            segments[segments.len() - 2],
            segments[segments.len() - 1]
        );
    }

    if segments.len() >= 2 {
        format!("{}:{}", segments[segments.len() - 2], segments[segments.len() - 1])
    } else {
        tail.to_string()
    }
}

fn default_physical_role(device: &DeviceInfo) -> String {
    if let Some((_, port)) = device.device_id.rsplit_once(":port") {
        return format!("Port {port}");
    }

    if device.device_id.starts_with("wireless:") {
        return wireless_cluster_role(device);
    }

    if device.has_pump {
        return "Pump device".to_string();
    }

    if device.has_lcd && device.has_fan {
        return "LCD cooling device".to_string();
    }

    if device.has_lcd {
        return "LCD device".to_string();
    }

    if device.has_fan && device.has_rgb {
        return "Cooling and lighting device".to_string();
    }

    if device.has_fan {
        return "Cooling device".to_string();
    }

    if device.has_rgb {
        return "Lighting device".to_string();
    }

    "Peripheral".to_string()
}

fn capability_summary(device: &DeviceInfo) -> String {
    let mut items = Vec::new();

    if device.has_rgb {
        items.push(match device.rgb_zone_count {
            Some(count) => format!("{count} RGB zone(s)"),
            None => "RGB control".to_string(),
        });
    }

    if device.has_fan {
        items.push(match device.fan_count {
            Some(count) => format!("{count} fan slot(s)"),
            None => "Fan control".to_string(),
        });
    }

    if device.has_lcd {
        items.push("LCD".to_string());
    }

    if device.has_pump {
        items.push("Pump telemetry".to_string());
    }

    if device.mb_sync_support {
        items.push("Motherboard sync".to_string());
    }

    if items.is_empty() {
        "Inventory only".to_string()
    } else {
        items.join(" | ")
    }
}

fn current_mode_summary(
    device: &DeviceInfo,
    online: bool,
    fan_rpms: &Option<Vec<u16>>,
    telemetry_fan_rpms: Option<&Vec<u16>>,
    telemetry: &TelemetrySnapshot,
) -> String {
    if !online {
        return offline_mode_summary(device, telemetry_fan_rpms);
    }

    let mut items = Vec::new();

    if device.has_rgb {
        items.push("Lighting ready".to_string());
    }

    if device.has_fan {
        if fan_rpms.as_ref().is_some_and(|rpms| !rpms.is_empty()) {
            items.push("Cooling telemetry live".to_string());
        } else {
            items.push("Cooling ready".to_string());
        }
    }

    if device.has_lcd {
        items.push(if telemetry.streaming_active {
            "LCD streaming active".to_string()
        } else {
            "LCD idle".to_string()
        });
    }

    if items.is_empty() {
        "Inventory only".to_string()
    } else {
        items.join(" | ")
    }
}

fn health_state(
    device: &DeviceInfo,
    online: bool,
    fan_rpms: &Option<Vec<u16>>,
    telemetry_fan_rpms: Option<&Vec<u16>>,
    coolant_temp: Option<f32>,
) -> DeviceHealthState {
    if !online {
        return DeviceHealthState {
            level: "offline".to_string(),
            summary: offline_health_summary(device, telemetry_fan_rpms),
        };
    }

    if device.has_fan {
        match fan_rpms {
            Some(rpms) if !rpms.is_empty() => {}
            _ => {
                return DeviceHealthState {
                    level: "warning".to_string(),
                    summary: "Fan telemetry missing".to_string(),
                }
            }
        }
    }

    if device.has_pump && coolant_temp.is_none() {
        return DeviceHealthState {
            level: "warning".to_string(),
            summary: "Pump telemetry missing".to_string(),
        };
    }

    DeviceHealthState {
        level: "healthy".to_string(),
        summary: "Device inventory healthy".to_string(),
    }
}

/// Tolerate up to this many consecutive missed wireless polls before
/// marking a device offline.  Must match the daemon's
/// `RPM_GRACE_MISSED_POLLS` constant so both layers agree on when
/// telemetry is still considered fresh.
const WIRELESS_ONLINE_GRACE_MISSED_POLLS: u8 = 2;

fn device_online(device: &DeviceInfo, telemetry_fan_rpms: &Option<Vec<u16>>) -> bool {
    if device.wireless_missed_polls.unwrap_or(0) > WIRELESS_ONLINE_GRACE_MISSED_POLLS {
        return false;
    }

    if device.wireless_channel.is_some()
        && device.has_fan
        && telemetry_fan_rpms
            .as_ref()
            .is_some_and(|rpms| !rpms.is_empty() && rpms.iter().all(|rpm| *rpm == 0))
    {
        return false;
    }

    true
}

fn offline_mode_summary(device: &DeviceInfo, telemetry_fan_rpms: Option<&Vec<u16>>) -> String {
    if device_is_powered_off(device, telemetry_fan_rpms) {
        if device.wireless_channel.is_some() {
            "Wireless device powered off".to_string()
        } else {
            "Device powered off".to_string()
        }
    } else if device.wireless_channel.is_some() {
        "Wireless device offline".to_string()
    } else {
        "Device offline".to_string()
    }
}

fn offline_health_summary(device: &DeviceInfo, telemetry_fan_rpms: Option<&Vec<u16>>) -> String {
    if device_is_powered_off(device, telemetry_fan_rpms) {
        if device.wireless_channel.is_some() {
            "Wireless device is connected but currently powered off".to_string()
        } else {
            "Device is currently powered off".to_string()
        }
    } else if device.wireless_channel.is_some() {
        "Wireless device not seen in the latest discovery poll".to_string()
    } else {
        "Device is offline".to_string()
    }
}

fn device_is_powered_off(device: &DeviceInfo, telemetry_fan_rpms: Option<&Vec<u16>>) -> bool {
    device.has_fan
        && telemetry_fan_rpms
            .is_some_and(|rpms| !rpms.is_empty() && rpms.iter().all(|rpm| *rpm == 0))
}

fn normalize_optional_value(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn unwrap_response<T: serde::de::DeserializeOwned>(response: IpcResponse) -> ApiResult<T> {
    match response {
        IpcResponse::Ok { data } => serde_json::from_value(data)
            .map_err(|e| crate::errors::ApiError::Internal(format!("response parse error: {e}"))),
        IpcResponse::Error { message } => Err(crate::errors::ApiError::DaemonMessage(message)),
    }
}

fn require_device(state: &AppState, id: &str) -> ApiResult<DeviceInfo> {
    let devices: Vec<DeviceInfo> = unwrap_response(state.daemon.send(&IpcRequest::ListDevices)?)?;
    devices
        .into_iter()
        .find(|d| d.device_id == id)
        .ok_or_else(|| crate::errors::ApiError::NotFound(format!("unknown device id: {id}")))
}

fn get_config(state: &AppState) -> ApiResult<AppConfig> {
    unwrap_response(state.daemon.send(&IpcRequest::GetConfig)?)
}

fn save_app_config(state: &AppState, cfg: AppConfig) -> ApiResult<()> {
    match state.daemon.send(&IpcRequest::SetConfig { config: cfg })? {
        IpcResponse::Ok { .. } => Ok(()),
        IpcResponse::Error { message } => Err(crate::errors::ApiError::DaemonMessage(message)),
    }
}

fn config_document_from_app_config(cfg: AppConfig) -> ConfigDocument {
    let lighting = lighting_document_from_rgb_config(cfg.rgb.clone().unwrap_or_default());
    let fans = fan_document_from_app_config(&cfg);
    let lcds = cfg
        .lcds
        .into_iter()
        .map(lcd_document_from_config)
        .collect();

    ConfigDocument {
        default_fps: cfg.default_fps,
        hid_driver: hid_driver_name(cfg.hid_driver).to_string(),
        lighting,
        fans,
        lcds,
    }
}

fn app_config_from_document(doc: ConfigDocument, daemon_config_path: &std::path::Path) -> ApiResult<AppConfig> {
    let ConfigDocument {
        default_fps,
        hid_driver,
        lighting,
        fans,
        lcds,
    } = doc;

    let fan_curves = fan_curves_from_document(&fans)?;
    let curve_names = fan_curves
        .iter()
        .map(|curve| curve.name.clone())
        .collect::<HashSet<_>>();

    let cfg = AppConfig {
        default_fps,
        hid_driver: parse_hid_driver(&hid_driver)?,
        lcds: lcds
            .into_iter()
            .map(lcd_config_from_document)
            .collect::<ApiResult<Vec<_>>>()?,
        fan_curves,
        fans: fan_config_from_document(fans, &curve_names)?,
        rgb: rgb_config_from_document(lighting)?,
    };

    validate_app_config(&cfg, daemon_config_path)?;
    Ok(cfg)
}

fn lighting_document_from_rgb_config(cfg: RgbAppConfig) -> LightingConfigDocument {
    let RgbAppConfig {
        enabled,
        openrgb_server,
        openrgb_port,
        global_led_zones,
        fan_led_zones,
        effect_route,
        devices,
    } = cfg;

    LightingConfigDocument {
        enabled,
        openrgb_server,
        openrgb_port,
        global_led_zones: document_led_zones_from_config(&global_led_zones),
        fan_led_zones: document_fan_led_zones_from_config(&fan_led_zones),
        effect_route: document_effect_route_from_config(&effect_route),
        devices: devices
            .into_iter()
            .map(|device| LightingDeviceConfigDocument {
                device_id: device.device_id,
                motherboard_sync: device.mb_rgb_sync,
                led_zones: device
                    .led_zones
                    .into_iter()
                    .map(|zone| LightingLedZoneConfigDocument {
                        zone: zone.zone_index,
                        led_indexes: zone.led_indexes,
                    })
                    .collect(),
                zones: device
                    .zones
                    .into_iter()
                    .map(|zone| LightingZoneConfigDocument {
                        zone: zone.zone_index,
                        effect: rgb_mode_name(zone.effect.mode),
                        colors: zone.effect.colors.into_iter().map(rgb_to_hex).collect(),
                        speed: zone.effect.speed,
                        brightness_percent: brightness_to_percent(zone.effect.brightness),
                        direction: rgb_direction_name(zone.effect.direction),
                        scope: rgb_scope_name(zone.effect.scope),
                        swap_left_right: zone.swap_lr,
                        swap_top_bottom: zone.swap_tb,
                        smoothness_ms: zone.effect.smoothness_ms,
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn rgb_config_from_document(doc: LightingConfigDocument) -> ApiResult<Option<RgbAppConfig>> {
    let LightingConfigDocument {
        enabled,
        openrgb_server,
        openrgb_port,
        global_led_zones,
        fan_led_zones,
        effect_route,
        devices,
    } = doc;

    let mut seen_global_led_zones = HashSet::new();
    let global_led_zones = global_led_zones
        .into_iter()
        .map(|zone| {
            if !seen_global_led_zones.insert(zone.zone) {
                return Err(crate::errors::ApiError::BadRequest(format!(
                    "duplicate global lighting led zone {}",
                    zone.zone
                )));
            }

            Ok(RgbLedZoneConfig {
                zone_index: zone.zone,
                led_indexes: zone.led_indexes,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;

    let mut seen_fan_led_zones = HashSet::new();
    let fan_led_zones = fan_led_zones
        .into_iter()
        .map(|fan| {
            if fan.device_id.trim().is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "lighting fan_led_zones device_id must not be empty".to_string(),
                ));
            }
            if fan.fan_index == 0 {
                return Err(crate::errors::ApiError::BadRequest(
                    "lighting fan_led_zones fan_index must be greater than zero".to_string(),
                ));
            }
            if !seen_fan_led_zones.insert((fan.device_id.clone(), fan.fan_index)) {
                return Err(crate::errors::ApiError::BadRequest(format!(
                    "duplicate lighting fan led layout for {} fan {}",
                    fan.device_id, fan.fan_index
                )));
            }

            let mut seen_zones = HashSet::new();
            let zones = fan
                .zones
                .into_iter()
                .map(|zone| {
                    if !seen_zones.insert(zone.zone) {
                        return Err(crate::errors::ApiError::BadRequest(format!(
                            "duplicate lighting led zone {} for {} fan {}",
                            zone.zone, fan.device_id, fan.fan_index
                        )));
                    }

                    Ok(RgbLedZoneConfig {
                        zone_index: zone.zone,
                        led_indexes: zone.led_indexes,
                    })
                })
                .collect::<ApiResult<Vec<_>>>()?;

            Ok(RgbFanLedZoneConfig {
                device_id: fan.device_id,
                fan_index: fan.fan_index,
                zones,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;

    let effect_route = effect_route_from_documents(effect_route)?;

    let mut seen_devices = HashSet::new();
    let devices = devices
        .into_iter()
        .map(|device| {
            if device.device_id.trim().is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "lighting device_id must not be empty".to_string(),
                ));
            }
            if !seen_devices.insert(device.device_id.clone()) {
                return Err(crate::errors::ApiError::BadRequest(format!(
                    "duplicate lighting device_id: {}",
                    device.device_id
                )));
            }

            let device_id = device.device_id.clone();
            let mut seen_led_zones = HashSet::new();
            let led_zones = device
                .led_zones
                .into_iter()
                .map(|zone| {
                    if !seen_led_zones.insert(zone.zone) {
                        return Err(crate::errors::ApiError::BadRequest(format!(
                            "duplicate lighting led zone {} for device {}",
                            zone.zone, device_id
                        )));
                    }

                    Ok(RgbLedZoneConfig {
                        zone_index: zone.zone,
                        led_indexes: zone.led_indexes,
                    })
                })
                .collect::<ApiResult<Vec<_>>>()?;
            let mut seen_zones = HashSet::new();
            let zones = device
                .zones
                .into_iter()
                .map(|zone| {
                    if !seen_zones.insert(zone.zone) {
                        return Err(crate::errors::ApiError::BadRequest(format!(
                            "duplicate lighting zone {} for device {}",
                            zone.zone, device_id
                        )));
                    }
                    if zone.speed > MAX_EFFECT_SPEED {
                        return Err(crate::errors::ApiError::BadRequest(
                            format!("lighting speed must be 0-{MAX_EFFECT_SPEED}"),
                        ));
                    }
                    if zone.brightness_percent > 100 {
                        return Err(crate::errors::ApiError::BadRequest(
                            "lighting brightness_percent must be 0-100".to_string(),
                        ));
                    }
                    if zone.colors.len() > 4 {
                        return Err(crate::errors::ApiError::BadRequest(
                            "lighting colors supports at most 4 entries per zone".to_string(),
                        ));
                    }

                    Ok(RgbZoneConfig {
                        zone_index: zone.zone,
                        effect: RgbEffect {
                            mode: parse_rgb_mode(&zone.effect)?,
                            colors: zone
                                .colors
                                .into_iter()
                                .map(|color| parse_hex_color(&color))
                                .collect::<ApiResult<Vec<_>>>()?,
                            speed: zone.speed,
                            brightness: percent_to_brightness(zone.brightness_percent),
                            direction: parse_direction(&zone.direction)?,
                            scope: parse_scope(&zone.scope)?,
                            smoothness_ms: zone.smoothness_ms,
                        },
                        swap_lr: zone.swap_left_right,
                        swap_tb: zone.swap_top_bottom,
                    })
                })
                .collect::<ApiResult<Vec<_>>>()?;

            Ok(RgbDeviceConfig {
                device_id,
                mb_rgb_sync: device.motherboard_sync,
                led_zones,
                zones,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;

    let rgb = RgbAppConfig {
        enabled,
        openrgb_server,
        openrgb_port,
        global_led_zones,
        fan_led_zones,
        effect_route,
        devices,
    };

    if rgb.enabled
        && !rgb.openrgb_server
        && rgb.openrgb_port == 6743
        && rgb.global_led_zones.is_empty()
        && rgb.fan_led_zones.is_empty()
        && rgb.effect_route.is_empty()
        && rgb.devices.is_empty()
    {
        Ok(None)
    } else {
        Ok(Some(rgb))
    }
}

fn fan_document_from_app_config(cfg: &AppConfig) -> FanConfigDocument {
    let fans = cfg.fans.clone().unwrap_or_else(default_fan_config);

    FanConfigDocument {
        update_interval_ms: fans.update_interval_ms,
        curves: cfg
            .fan_curves
            .iter()
            .cloned()
            .map(|curve| FanCurveDocument {
                name: curve.name,
                temperature_source: curve.temp_command,
                points: curve
                    .curve
                    .into_iter()
                    .map(|(temperature_celsius, percent)| FanCurvePointDocument {
                        temperature_celsius,
                        percent,
                    })
                    .collect(),
            })
            .collect(),
        devices: fans
            .speeds
            .into_iter()
            .map(|device| FanDeviceConfigDocument {
                device_id: device.device_id,
                slots: device
                    .speeds
                    .into_iter()
                    .enumerate()
                    .map(|(idx, speed)| fan_slot_document_from_speed((idx + 1) as u8, speed))
                    .collect(),
            })
            .collect(),
    }
}

fn fan_curves_from_document(doc: &FanConfigDocument) -> ApiResult<Vec<FanCurve>> {
    let mut names = HashSet::new();
    let mut curves = Vec::new();

    for curve in &doc.curves {
        if curve.name.trim().is_empty() {
            return Err(crate::errors::ApiError::BadRequest(
                "fan curve name must not be empty".to_string(),
            ));
        }
        if !names.insert(curve.name.clone()) {
            return Err(crate::errors::ApiError::BadRequest(format!(
                "duplicate fan curve name: {}",
                curve.name
            )));
        }
        if curve.temperature_source.trim().is_empty() {
            return Err(crate::errors::ApiError::BadRequest(format!(
                "fan curve '{}' requires a temperature_source",
                curve.name
            )));
        }
        if curve.points.is_empty() {
            return Err(crate::errors::ApiError::BadRequest(format!(
                "fan curve '{}' requires at least one point",
                curve.name
            )));
        }

        let points = curve
            .points
            .iter()
            .map(|point| {
                if !point.temperature_celsius.is_finite() {
                    return Err(crate::errors::ApiError::BadRequest(format!(
                        "fan curve '{}' contains a non-finite temperature",
                        curve.name
                    )));
                }
                if !point.percent.is_finite() || !(0.0..=100.0).contains(&point.percent) {
                    return Err(crate::errors::ApiError::BadRequest(format!(
                        "fan curve '{}' percent must be between 0 and 100",
                        curve.name
                    )));
                }
                Ok((point.temperature_celsius, point.percent))
            })
            .collect::<ApiResult<Vec<_>>>()?;

        curves.push(FanCurve {
            name: curve.name.clone(),
            temp_command: curve.temperature_source.clone(),
            curve: points,
        });
    }

    Ok(curves)
}

fn fan_config_from_document(
    doc: FanConfigDocument,
    curve_names: &HashSet<String>,
) -> ApiResult<Option<FanConfig>> {
    let FanConfigDocument {
        update_interval_ms,
        curves: _,
        devices: input_devices,
    } = doc;

    if update_interval_ms == 0 {
        return Err(crate::errors::ApiError::BadRequest(
            "fans.update_interval_ms must be greater than zero".to_string(),
        ));
    }

    let mut devices = Vec::new();
    for device in input_devices {
        if let Some(device_id) = &device.device_id {
            if device_id.trim().is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "fan device_id must not be empty".to_string(),
                ));
            }
        }
        if device.slots.len() != 4 {
            return Err(crate::errors::ApiError::BadRequest(
                "fan devices must define exactly 4 slots".to_string(),
            ));
        }

        let mut speeds = std::array::from_fn(|_| None::<FanSpeed>);
        for slot in device.slots {
            if !(1..=4).contains(&slot.slot) {
                return Err(crate::errors::ApiError::BadRequest(
                    "fan slot must be 1-4".to_string(),
                ));
            }
            let idx = (slot.slot - 1) as usize;
            if speeds[idx].is_some() {
                return Err(crate::errors::ApiError::BadRequest(format!(
                    "duplicate fan slot {}",
                    slot.slot
                )));
            }
            speeds[idx] = Some(fan_speed_from_document(slot, curve_names)?);
        }

        if speeds.iter().any(|speed| speed.is_none()) {
            return Err(crate::errors::ApiError::BadRequest(
                "fan devices must include slots 1 through 4".to_string(),
            ));
        }

        devices.push(FanGroup {
            device_id: device.device_id,
            speeds: speeds.map(|speed| speed.expect("validated fan slot")),
        });
    }

    if devices.is_empty() && update_interval_ms == 1000 {
        Ok(None)
    } else {
        Ok(Some(FanConfig {
            speeds: devices,
            update_interval_ms,
        }))
    }
}

fn fan_slot_document_from_speed(slot: u8, speed: FanSpeed) -> FanSlotConfigDocument {
    match speed {
        FanSpeed::Constant(pwm) => FanSlotConfigDocument {
            slot,
            mode: "manual".to_string(),
            percent: Some(pwm_to_percent(pwm)),
            curve: None,
        },
        FanSpeed::Curve(name) if name == MB_SYNC_KEY => FanSlotConfigDocument {
            slot,
            mode: "mb_sync".to_string(),
            percent: None,
            curve: None,
        },
        FanSpeed::Curve(name) => FanSlotConfigDocument {
            slot,
            mode: "curve".to_string(),
            percent: None,
            curve: Some(name),
        },
    }
}

fn fan_speed_from_document(
    slot: FanSlotConfigDocument,
    curve_names: &HashSet<String>,
) -> ApiResult<FanSpeed> {
    match slot.mode.to_lowercase().as_str() {
        "manual" => {
            let percent = slot.percent.ok_or_else(|| {
                crate::errors::ApiError::BadRequest(format!(
                    "fan slot {} in manual mode requires percent",
                    slot.slot
                ))
            })?;
            if percent > 100 {
                return Err(crate::errors::ApiError::BadRequest(format!(
                    "fan slot {} percent must be 0-100",
                    slot.slot
                )));
            }
            Ok(FanSpeed::Constant(percent_to_pwm(percent)))
        }
        "curve" => {
            let curve = slot.curve.ok_or_else(|| {
                crate::errors::ApiError::BadRequest(format!(
                    "fan slot {} in curve mode requires curve",
                    slot.slot
                ))
            })?;
            if !curve_names.contains(&curve) {
                return Err(crate::errors::ApiError::BadRequest(format!(
                    "fan slot {} references unknown curve '{}'",
                    slot.slot, curve
                )));
            }
            Ok(FanSpeed::Curve(curve))
        }
        "mb_sync" => Ok(FanSpeed::Curve(MB_SYNC_KEY.to_string())),
        other => Err(crate::errors::ApiError::BadRequest(format!(
            "unknown fan mode: {other}"
        ))),
    }
}

fn lcd_document_from_config(cfg: LcdConfig) -> LcdConfigDocument {
    LcdConfigDocument {
        device_id: Some(cfg.device_id()),
        index: cfg.index,
        serial: cfg.serial,
        media: media_type_name(cfg.media_type).to_string(),
        path: cfg.path.map(|path| path.display().to_string()),
        fps: cfg.fps,
        color: cfg.rgb.map(rgb_to_hex),
        orientation: cfg.orientation,
        sensor: cfg.sensor.map(sensor_document_from_descriptor),
    }
}

fn lcd_config_from_document(doc: LcdConfigDocument) -> ApiResult<LcdConfig> {
    let media_type = parse_media_type(&doc.media)?;
    let rgb = match &doc.color {
        Some(color) => Some(parse_hex_color(color)?),
        None => None,
    };

    Ok(LcdConfig {
        index: doc.index,
        serial: doc.serial,
        media_type,
        path: doc.path.map(PathBuf::from),
        fps: doc.fps,
        rgb,
        orientation: normalize_orientation(doc.orientation),
        sensor: doc
            .sensor
            .map(sensor_descriptor_from_document)
            .transpose()?,
    })
}

fn sensor_document_from_descriptor(sensor: SensorDescriptor) -> SensorConfigDocument {
    SensorConfigDocument {
        label: sensor.label,
        unit: sensor.unit,
        source: sensor_source_document_from_config(sensor.source),
        text_color: rgb_to_hex(sensor.text_color),
        background_color: rgb_to_hex(sensor.background_color),
        gauge_background_color: rgb_to_hex(sensor.gauge_background_color),
        ranges: sensor
            .gauge_ranges
            .into_iter()
            .map(|range| SensorRangeDocument {
                max: range.max,
                color: rgb_to_hex(range.color),
            })
            .collect(),
        update_interval_ms: sensor.update_interval_ms,
        gauge_start_angle: sensor.gauge_start_angle,
        gauge_sweep_angle: sensor.gauge_sweep_angle,
        gauge_outer_radius: sensor.gauge_outer_radius,
        gauge_thickness: sensor.gauge_thickness,
        bar_corner_radius: sensor.bar_corner_radius,
        value_font_size: sensor.value_font_size,
        unit_font_size: sensor.unit_font_size,
        label_font_size: sensor.label_font_size,
        font_path: sensor.font_path.map(|path| path.display().to_string()),
        decimal_places: sensor.decimal_places,
        value_offset: sensor.value_offset,
        unit_offset: sensor.unit_offset,
        label_offset: sensor.label_offset,
    }
}

fn sensor_descriptor_from_document(doc: SensorConfigDocument) -> ApiResult<SensorDescriptor> {
    Ok(SensorDescriptor {
        label: doc.label,
        unit: doc.unit,
        source: sensor_source_config_from_document(doc.source),
        text_color: parse_hex_color(&doc.text_color)?,
        background_color: parse_hex_color(&doc.background_color)?,
        gauge_background_color: parse_hex_color(&doc.gauge_background_color)?,
        gauge_ranges: doc
            .ranges
            .into_iter()
            .map(|range| {
                Ok(SensorRange {
                    max: range.max,
                    color: parse_hex_color(&range.color)?,
                })
            })
            .collect::<ApiResult<Vec<_>>>()?,
        update_interval_ms: doc.update_interval_ms,
        gauge_start_angle: doc.gauge_start_angle,
        gauge_sweep_angle: doc.gauge_sweep_angle,
        gauge_outer_radius: doc.gauge_outer_radius,
        gauge_thickness: doc.gauge_thickness,
        bar_corner_radius: doc.bar_corner_radius,
        value_font_size: doc.value_font_size,
        unit_font_size: doc.unit_font_size,
        label_font_size: doc.label_font_size,
        font_path: doc.font_path.map(PathBuf::from),
        decimal_places: doc.decimal_places,
        value_offset: doc.value_offset,
        unit_offset: doc.unit_offset,
        label_offset: doc.label_offset,
    })
}

fn sensor_source_document_from_config(source: SensorSourceConfig) -> SensorSourceDocument {
    match source {
        SensorSourceConfig::Constant { value } => SensorSourceDocument::Constant { value },
        SensorSourceConfig::Command { cmd } => SensorSourceDocument::Command { command: cmd },
    }
}

fn sensor_source_config_from_document(source: SensorSourceDocument) -> SensorSourceConfig {
    match source {
        SensorSourceDocument::Constant { value } => SensorSourceConfig::Constant { value },
        SensorSourceDocument::Command { command } => SensorSourceConfig::Command { cmd: command },
    }
}

fn validate_app_config(cfg: &AppConfig, daemon_config_path: &std::path::Path) -> ApiResult<()> {
    if cfg.default_fps <= 0.0 {
        return Err(crate::errors::ApiError::BadRequest(
            "default_fps must be greater than zero".to_string(),
        ));
    }

    let curve_names = cfg
        .fan_curves
        .iter()
        .map(|curve| curve.name.clone())
        .collect::<HashSet<_>>();

    if let Some(fans) = &cfg.fans {
        if fans.update_interval_ms == 0 {
            return Err(crate::errors::ApiError::BadRequest(
                "fans.update_interval_ms must be greater than zero".to_string(),
            ));
        }

        for group in &fans.speeds {
            if let Some(device_id) = &group.device_id {
                if device_id.trim().is_empty() {
                    return Err(crate::errors::ApiError::BadRequest(
                        "fan device_id must not be empty".to_string(),
                    ));
                }
            }

            for speed in &group.speeds {
                if let FanSpeed::Curve(name) = speed {
                    if name != MB_SYNC_KEY && !curve_names.contains(name) {
                        return Err(crate::errors::ApiError::BadRequest(format!(
                            "fan config references unknown curve '{}'",
                            name
                        )));
                    }
                }
            }
        }
    }

    if let Some(rgb) = &cfg.rgb {
        let mut seen_global_led_zones = HashSet::new();
        for led_zone in &rgb.global_led_zones {
            if !seen_global_led_zones.insert(led_zone.zone_index) {
                return Err(crate::errors::ApiError::BadRequest(format!(
                    "duplicate global lighting led zone {}",
                    led_zone.zone_index
                )));
            }
        }

        let mut seen_fan_led_zones = HashSet::new();
        for fan_layout in &rgb.fan_led_zones {
            if fan_layout.device_id.trim().is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "lighting fan_led_zones device_id must not be empty".to_string(),
                ));
            }
            if fan_layout.fan_index == 0 {
                return Err(crate::errors::ApiError::BadRequest(
                    "lighting fan_led_zones fan_index must be greater than zero".to_string(),
                ));
            }
            if !seen_fan_led_zones.insert((fan_layout.device_id.clone(), fan_layout.fan_index)) {
                return Err(crate::errors::ApiError::BadRequest(format!(
                    "duplicate lighting fan led layout for {} fan {}",
                    fan_layout.device_id, fan_layout.fan_index
                )));
            }

            let mut seen_led_zones = HashSet::new();
            for led_zone in &fan_layout.zones {
                if !seen_led_zones.insert(led_zone.zone_index) {
                    return Err(crate::errors::ApiError::BadRequest(format!(
                        "duplicate lighting led zone {} for {} fan {}",
                        led_zone.zone_index, fan_layout.device_id, fan_layout.fan_index
                    )));
                }
            }
        }

        let mut seen_effect_route = HashSet::new();
        for entry in &rgb.effect_route {
            if entry.device_id.trim().is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "lighting effect_route device_id must not be empty".to_string(),
                ));
            }
            if entry.fan_index == 0 {
                return Err(crate::errors::ApiError::BadRequest(
                    "lighting effect_route fan_index must be greater than zero".to_string(),
                ));
            }
            if !seen_effect_route.insert((entry.device_id.clone(), entry.fan_index)) {
                return Err(crate::errors::ApiError::BadRequest(format!(
                    "duplicate lighting effect route entry for {} fan {}",
                    entry.device_id, entry.fan_index
                )));
            }
        }

        for device in &rgb.devices {
            if device.device_id.trim().is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "lighting device_id must not be empty".to_string(),
                ));
            }

            let mut seen_led_zones = HashSet::new();
            for led_zone in &device.led_zones {
                if !seen_led_zones.insert(led_zone.zone_index) {
                    return Err(crate::errors::ApiError::BadRequest(format!(
                        "duplicate lighting led zone {} for device {}",
                        led_zone.zone_index, device.device_id
                    )));
                }
            }

            for zone in &device.zones {
                if zone.effect.speed > MAX_EFFECT_SPEED {
                    return Err(crate::errors::ApiError::BadRequest(
                        format!("lighting speed must be 0-{MAX_EFFECT_SPEED}"),
                    ));
                }
                if zone.effect.brightness > 4 {
                    return Err(crate::errors::ApiError::BadRequest(
                        "lighting brightness must map to 0-4".to_string(),
                    ));
                }
                if zone.effect.colors.len() > 4 {
                    return Err(crate::errors::ApiError::BadRequest(
                        "lighting colors supports at most 4 entries per zone".to_string(),
                    ));
                }
            }
        }
    }

    let base_dir = daemon_config_path
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let mut lcd_keys = HashSet::new();

    for lcd in &cfg.lcds {
        let mut resolved = lcd.clone();
        if let Some(path) = &resolved.path {
            if path.is_relative() {
                resolved.path = Some(base_dir.join(path));
            }
        }
        if let Some(sensor) = &mut resolved.sensor {
            if let Some(font_path) = &sensor.font_path {
                if font_path.is_relative() {
                    sensor.font_path = Some(base_dir.join(font_path));
                }
            }
        }

        let device_key = lcd.device_id();
        if !lcd_keys.insert(device_key.clone()) {
            return Err(crate::errors::ApiError::BadRequest(format!(
                "duplicate LCD target '{}'",
                device_key
            )));
        }
        resolved.validate().map_err(|err| {
            crate::errors::ApiError::BadRequest(format!(
                "invalid LCD config for '{}': {}",
                device_key, err
            ))
        })?;
    }

    Ok(())
}

fn profile_document_from_upsert(
    profile: ProfileUpsertDocument,
    existing_metadata: Option<ProfileMetadataDocument>,
) -> ApiResult<ProfileDocument> {
    let normalized_targets = normalize_profile_targets(profile.targets.clone());
    let normalized_profile = ProfileUpsertDocument {
        targets: normalized_targets.clone(),
        ..profile
    };
    validate_profile_upsert(&normalized_profile)?;

    let now = timestamp_now()?;
    let metadata = ProfileMetadataDocument {
        created_at: existing_metadata
            .as_ref()
            .map(|metadata| metadata.created_at.clone())
            .unwrap_or_else(|| now.clone()),
        updated_at: now,
    };

    Ok(ProfileDocument {
        id: normalized_profile.id,
        name: normalized_profile.name.trim().to_string(),
        description: normalized_profile
            .description
            .map(|description| description.trim().to_string())
            .filter(|description| !description.is_empty()),
        targets: normalized_targets,
        lighting: normalized_profile.lighting,
        fans: normalized_profile.fans,
        metadata,
    })
}

fn validate_profile_upsert(profile: &ProfileUpsertDocument) -> ApiResult<()> {
    if !is_valid_profile_id(&profile.id) {
        return Err(crate::errors::ApiError::BadRequest(
            "profile id must contain only lowercase letters, numbers, '-' or '_'".to_string(),
        ));
    }

    if profile.name.trim().is_empty() {
        return Err(crate::errors::ApiError::BadRequest(
            "profile name must not be empty".to_string(),
        ));
    }

    validate_profile_targets(&profile.targets)?;

    if profile.lighting.is_none() && profile.fans.is_none() {
        return Err(crate::errors::ApiError::BadRequest(
            "profile must include lighting and/or fans".to_string(),
        ));
    }

    if let Some(lighting) = &profile.lighting {
        validate_profile_lighting(lighting)?;
    }

    if let Some(fans) = &profile.fans {
        validate_profile_fans(fans)?;
    }

    Ok(())
}

fn is_valid_profile_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

fn normalize_profile_targets(targets: ProfileTargetsDocument) -> ProfileTargetsDocument {
    let mut device_ids = targets
        .device_ids
        .into_iter()
        .map(|device_id| device_id.trim().to_string())
        .filter(|device_id| !device_id.is_empty())
        .collect::<Vec<_>>();
    device_ids.sort();
    device_ids.dedup();

    ProfileTargetsDocument {
        mode: targets.mode.trim().to_lowercase(),
        device_ids,
    }
}

fn validate_profile_targets(targets: &ProfileTargetsDocument) -> ApiResult<()> {
    match targets.mode.trim().to_lowercase().as_str() {
        "all" => {
            if !targets.device_ids.is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "targets.device_ids must be empty when mode is 'all'".to_string(),
                ));
            }
        }
        "devices" => {
            if targets.device_ids.is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "targets.device_ids must not be empty when mode is 'devices'".to_string(),
                ));
            }
        }
        other => {
            return Err(crate::errors::ApiError::BadRequest(format!(
                "unknown targets mode: {other}"
            )))
        }
    }

    Ok(())
}

fn validate_profile_lighting(lighting: &ProfileLightingDocument) -> ApiResult<()> {
    if let Some(color) = &lighting.color {
        parse_hex_color(color)?;
    }

    if let Some(effect) = &lighting.effect {
        parse_rgb_mode(effect)?;
    }

    if let Some(brightness_percent) = lighting.brightness_percent {
        if brightness_percent > 100 {
            return Err(crate::errors::ApiError::BadRequest(
                "profile lighting brightness_percent must be 0-100".to_string(),
            ));
        }
    }

    if let Some(speed) = lighting.speed {
        if speed > MAX_EFFECT_SPEED {
            return Err(crate::errors::ApiError::BadRequest(
                format!("profile lighting speed must be 0-{MAX_EFFECT_SPEED}"),
            ));
        }
    }

    if let Some(direction) = &lighting.direction {
        parse_direction(direction)?;
    }

    if let Some(scope) = &lighting.scope {
        parse_scope(scope)?;
    }

    Ok(())
}

fn validate_profile_fans(fans: &ProfileFanDocument) -> ApiResult<()> {
    match fans.mode.trim().to_lowercase().as_str() {
        "manual" => {
            let Some(percent) = fans.percent else {
                return Err(crate::errors::ApiError::BadRequest(
                    "profile fans.percent is required for manual mode".to_string(),
                ));
            };
            if percent > 100 {
                return Err(crate::errors::ApiError::BadRequest(
                    "profile fan percent must be 0-100".to_string(),
                ));
            }
        }
        other => {
            return Err(crate::errors::ApiError::BadRequest(format!(
                "unsupported profile fan mode: {other}"
            )))
        }
    }

    Ok(())
}

fn timestamp_now() -> ApiResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|err| crate::errors::ApiError::Internal(format!("timestamp format error: {err}")))
}

fn resolve_profile_targets<'a>(
    targets: &ProfileTargetsDocument,
    devices: &'a [DeviceInfo],
) -> ApiResult<Vec<&'a DeviceInfo>> {
    match targets.mode.as_str() {
        "all" => Ok(devices.iter().collect()),
        "devices" => targets
            .device_ids
            .iter()
            .map(|device_id| {
                devices.iter().find(|device| &device.device_id == device_id).ok_or_else(|| {
                    crate::errors::ApiError::NotFound(format!(
                        "profile target device not found: {device_id}"
                    ))
                })
            })
            .collect(),
        other => Err(crate::errors::ApiError::BadRequest(format!(
            "unknown targets mode: {other}"
        ))),
    }
}

fn apply_profile_lighting(
    cfg: &mut AppConfig,
    devices: &[&DeviceInfo],
    lighting: &ProfileLightingDocument,
    applied_devices: &mut Vec<String>,
    skipped_devices: &mut Vec<ProfileApplySkip>,
) -> ApiResult<()> {
    let rgb = cfg.rgb.get_or_insert_with(RgbAppConfig::default);
    let target_mode = if lighting.enabled {
        if let Some(effect) = &lighting.effect {
            Some(parse_rgb_mode(effect)?)
        } else if lighting.color.is_some() {
            Some(RgbMode::Static)
        } else {
            None
        }
    } else {
        Some(RgbMode::Off)
    };

    for device in devices {
        if !device.has_rgb {
            skipped_devices.push(ProfileApplySkip {
                device_id: device.device_id.clone(),
                section: "lighting".to_string(),
                reason: "device has no RGB capability".to_string(),
            });
            continue;
        }

        prune_lighting_zones(rgb, device);
        let zone_indexes = existing_or_default_zone_indexes(rgb, device);
        for zone in zone_indexes {
            let mut effect = effect_for_zone(rgb, &device.device_id, zone);
            if let Some(mode) = target_mode {
                effect.mode = mode;
            }

            if let Some(color) = &lighting.color {
                effect.colors = vec![parse_hex_color(color)?];
            } else if !lighting.enabled {
                effect.colors.clear();
            }

            if let Some(brightness_percent) = lighting.brightness_percent {
                effect.brightness = percent_to_brightness(brightness_percent);
            }

            if let Some(speed) = lighting.speed {
                effect.speed = speed;
            }

            if let Some(direction) = &lighting.direction {
                effect.direction = parse_direction(direction)?;
            }

            if let Some(scope) = &lighting.scope {
                effect.scope = parse_scope(scope)?;
            }

            upsert_zone_effect(rgb, &device.device_id, zone, effect);
        }

        applied_devices.push(device.device_id.clone());
    }

    Ok(())
}

fn apply_profile_fans(
    cfg: &mut AppConfig,
    devices: &[&DeviceInfo],
    fans: &ProfileFanDocument,
    applied_devices: &mut Vec<String>,
    skipped_devices: &mut Vec<ProfileApplySkip>,
) -> ApiResult<()> {
    let fan_cfg = cfg.fans.get_or_insert_with(default_fan_config);
    let percent = if fans.enabled {
        fans.percent.unwrap_or(0)
    } else {
        0
    };
    let pwm = percent_to_pwm(percent);

    for device in devices {
        if !device.has_fan {
            skipped_devices.push(ProfileApplySkip {
                device_id: device.device_id.clone(),
                section: "fans".to_string(),
                reason: "device has no fan capability".to_string(),
            });
            continue;
        }

        let group = match fan_cfg
            .speeds
            .iter_mut()
            .find(|group| group.device_id.as_deref() == Some(device.device_id.as_str()))
        {
            Some(group) => group,
            None => {
                fan_cfg.speeds.push(FanGroup {
                    device_id: Some(device.device_id.clone()),
                    speeds: default_fan_speeds(pwm),
                });
                fan_cfg.speeds.last_mut().unwrap()
            }
        };

        group.speeds = default_fan_speeds(pwm);
        applied_devices.push(device.device_id.clone());
    }

    Ok(())
}

fn existing_or_default_zone_indexes(cfg: &RgbAppConfig, device: &DeviceInfo) -> Vec<u8> {
    effective_zone_indexes(cfg, device)
}

fn save_rgb_config(state: &AppState, cfg: RgbAppConfig) -> ApiResult<()> {
    match state.daemon.send(&IpcRequest::SetRgbConfig { config: cfg })? {
        IpcResponse::Ok { .. } => Ok(()),
        IpcResponse::Error { message } => Err(crate::errors::ApiError::DaemonMessage(message)),
    }
}

fn save_fan_config(state: &AppState, cfg: FanConfig) -> ApiResult<()> {
    match state.daemon.send(&IpcRequest::SetFanConfig { config: cfg })? {
        IpcResponse::Ok { .. } => Ok(()),
        IpcResponse::Error { message } => Err(crate::errors::ApiError::DaemonMessage(message)),
    }
}

fn document_led_zones_from_config(led_zones: &[RgbLedZoneConfig]) -> Vec<LightingLedZoneConfigDocument> {
    let mut zones = led_zones
        .iter()
        .map(|zone| LightingLedZoneConfigDocument {
            zone: zone.zone_index,
            led_indexes: zone.led_indexes.clone(),
        })
        .collect::<Vec<_>>();
    zones.sort_by_key(|zone| zone.zone);
    zones
}

fn document_fan_led_zones_from_config(
    fan_led_zones: &[RgbFanLedZoneConfig],
) -> Vec<LightingFanLedZoneConfigDocument> {
    let mut documents = fan_led_zones
        .iter()
        .map(|fan| LightingFanLedZoneConfigDocument {
            device_id: fan.device_id.clone(),
            fan_index: fan.fan_index,
            zones: document_led_zones_from_config(&fan.zones),
        })
        .collect::<Vec<_>>();
    documents.sort_by(|left, right| {
        left.device_id
            .cmp(&right.device_id)
            .then(left.fan_index.cmp(&right.fan_index))
    });
    documents
}

fn document_effect_route_from_config(
    effect_route: &[RgbEffectRouteEntry],
) -> Vec<LightingEffectRouteEntryDocument> {
    effect_route
        .iter()
        .map(|entry| LightingEffectRouteEntryDocument {
            device_id: entry.device_id.clone(),
            fan_index: entry.fan_index,
        })
        .collect()
}

fn effect_route_from_documents(
    route: Vec<LightingEffectRouteEntryDocument>,
) -> ApiResult<Vec<RgbEffectRouteEntry>> {
    let mut seen_entries = HashSet::new();

    route
        .into_iter()
        .map(|entry| {
            if entry.device_id.trim().is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "lighting effect_route device_id must not be empty".to_string(),
                ));
            }
            if entry.fan_index == 0 {
                return Err(crate::errors::ApiError::BadRequest(
                    "lighting effect_route fan_index must be greater than zero".to_string(),
                ));
            }
            if !seen_entries.insert((entry.device_id.clone(), entry.fan_index)) {
                return Err(crate::errors::ApiError::BadRequest(format!(
                    "duplicate lighting effect route entry for {} fan {}",
                    entry.device_id, entry.fan_index
                )));
            }

            Ok(RgbEffectRouteEntry {
                device_id: entry.device_id,
                fan_index: entry.fan_index,
            })
        })
        .collect()
}

fn legacy_device_led_zone_layout(
    cfg: &RgbAppConfig,
    device_id: &str,
) -> Option<Vec<LightingLedZoneConfigDocument>> {
    let configured_device = cfg
        .devices
        .iter()
        .find(|configured_device| configured_device.device_id == device_id)?;
    if configured_device.led_zones.is_empty() {
        return None;
    }

    Some(document_led_zones_from_config(&configured_device.led_zones))
}

fn legacy_cluster_led_zone_layout(
    cfg: &RgbAppConfig,
    device_id: &str,
) -> Option<Vec<LightingLedZoneConfigDocument>> {
    if !cfg.global_led_zones.is_empty() {
        return Some(document_led_zones_from_config(&cfg.global_led_zones));
    }
    if let Some(zones) = legacy_device_led_zone_layout(cfg, device_id) {
        return Some(zones);
    }
    cfg.devices
        .iter()
        .find(|configured_device| !configured_device.led_zones.is_empty())
        .map(|configured_device| document_led_zones_from_config(&configured_device.led_zones))
}

fn configured_led_zone_indexes(cfg: &RgbAppConfig, device: &DeviceInfo) -> Option<Vec<u8>> {
    let mut zones = cfg
        .fan_led_zones
        .iter()
        .filter(|configured_fan| configured_fan.device_id == device.device_id)
        .flat_map(|configured_fan| configured_fan.zones.iter().map(|zone| zone.zone_index))
        .collect::<Vec<_>>();
    if zones.is_empty() {
        let led_zones = if device_uses_cluster_rgb_zones(device) {
            legacy_cluster_led_zone_layout(cfg, &device.device_id)?
        } else {
            legacy_device_led_zone_layout(cfg, &device.device_id)?
        };
        zones = led_zones.into_iter().map(|zone| zone.zone).collect::<Vec<_>>();
    }
    zones.sort_unstable();
    zones.dedup();
    Some(zones)
}

fn effective_zone_indexes(cfg: &RgbAppConfig, device: &DeviceInfo) -> Vec<u8> {
    if let Some(zones) = configured_led_zone_indexes(cfg, device) {
        return zones;
    }

    if let Some(zone_count) = device.rgb_zone_count {
        if zone_count > 0 {
            return (0..zone_count).collect();
        }
    }

    let Some(configured_device) = cfg
        .devices
        .iter()
        .find(|configured_device| configured_device.device_id == device.device_id)
    else {
        return vec![0];
    };

    if configured_device.zones.is_empty() {
        vec![0]
    } else {
        let mut zones = configured_device
            .zones
            .iter()
            .map(|zone| zone.zone_index)
            .collect::<Vec<_>>();
        zones.sort_unstable();
        zones.dedup();
        zones
    }
}

fn device_uses_cluster_rgb_zones(device: &DeviceInfo) -> bool {
    device.wireless_channel.is_some() && matches!(device.family, DeviceFamily::SlInf)
}

fn normalize_lighting_effect_for_device(device: &DeviceInfo, effect: &mut RgbEffect) {
    if device_uses_cluster_rgb_zones(device) {
        effect.scope = RgbScope::All;
    }
}

fn prune_lighting_zones(cfg: &mut RgbAppConfig, device: &DeviceInfo) {
    let valid_zones = effective_zone_indexes(cfg, device)
        .into_iter()
        .collect::<HashSet<_>>();
    if valid_zones.is_empty() {
        return;
    }

    let Some(configured_device) = cfg
        .devices
        .iter_mut()
        .find(|configured_device| configured_device.device_id == device.device_id)
    else {
        return;
    };

    configured_device
        .zones
        .retain(|zone| valid_zones.contains(&zone.zone_index));
    configured_device.zones.sort_by_key(|zone| zone.zone_index);
    configured_device
        .zones
        .dedup_by_key(|zone| zone.zone_index);
}

fn build_lighting_zones(cfg: &RgbAppConfig, device: &DeviceInfo) -> Vec<LightingZoneState> {
    existing_or_default_zone_indexes(cfg, device)
        .into_iter()
        .map(|zone_index| {
            let mut effect = effect_for_zone(cfg, &device.device_id, zone_index);
            normalize_lighting_effect_for_device(device, &mut effect);
            LightingZoneState {
                zone: zone_index,
                effect: rgb_mode_name(effect.mode),
                colors: effect.colors.iter().map(|color| rgb_to_hex(*color)).collect(),
                speed: effect.speed,
                brightness_percent: brightness_to_percent(effect.brightness),
                direction: rgb_direction_name(effect.direction),
                scope: rgb_scope_name(effect.scope),
                smoothness_ms: effect.smoothness_ms,
            }
        })
        .collect()
}

fn effect_for_zone(cfg: &RgbAppConfig, device_id: &str, zone: u8) -> RgbEffect {
    if let Some(dev) = cfg.devices.iter().find(|d| d.device_id == device_id) {
        if let Some(z) = dev.zones.iter().find(|z| z.zone_index == zone) {
            return z.effect.clone();
        }
    }
    RgbEffect {
        mode: RgbMode::Static,
        colors: vec![[255, 255, 255]],
        speed: 2,
        brightness: 4,
        direction: RgbDirection::Clockwise,
        scope: RgbScope::All,
        smoothness_ms: 0,
    }
}

fn upsert_zone_effect(cfg: &mut RgbAppConfig, device_id: &str, zone: u8, effect: RgbEffect) {
    let dev = match cfg.devices.iter_mut().find(|d| d.device_id == device_id) {
        Some(d) => d,
        None => {
            cfg.devices.push(RgbDeviceConfig {
                device_id: device_id.to_string(),
                mb_rgb_sync: false,
                led_zones: Vec::new(),
                zones: Vec::new(),
            });
            cfg.devices.last_mut().unwrap()
        }
    };

    if let Some(z) = dev.zones.iter_mut().find(|z| z.zone_index == zone) {
        z.effect = effect;
        return;
    }

    dev.zones.push(RgbZoneConfig {
        zone_index: zone,
        effect,
        swap_lr: false,
        swap_tb: false,
    });
}

fn fan_slots_from_config(cfg: &AppConfig, device_id: &str) -> (Vec<FanSlotState>, u64) {
    let default_interval = 1000;
    let Some(fans) = cfg.fans.as_ref() else {
        return (Vec::new(), default_interval);
    };
    let group = fans
        .speeds
        .iter()
        .find(|g| g.device_id.as_deref() == Some(device_id));

    let update_interval = fans.update_interval_ms;
    let Some(group) = group else {
        return (Vec::new(), update_interval);
    };

    let mut slots = Vec::new();
    for (idx, speed) in group.speeds.iter().enumerate() {
        let slot = (idx + 1) as u8;
        let state = match speed {
            FanSpeed::Constant(pwm) => FanSlotState {
                slot,
                mode: "manual".to_string(),
                percent: Some(pwm_to_percent(*pwm)),
                pwm: Some(*pwm),
                curve: None,
            },
            FanSpeed::Curve(name) if name == MB_SYNC_KEY => FanSlotState {
                slot,
                mode: "mb_sync".to_string(),
                percent: None,
                pwm: None,
                curve: None,
            },
            FanSpeed::Curve(name) => FanSlotState {
                slot,
                mode: "curve".to_string(),
                percent: None,
                pwm: None,
                curve: Some(name.clone()),
            },
        };
        slots.push(state);
    }

    (slots, update_interval)
}

fn build_fan_state_response(
    cfg: &AppConfig,
    telemetry: &TelemetrySnapshot,
    device: &DeviceInfo,
) -> FanStateResponse {
    let (mut slots, update_interval_ms) = fan_slots_from_config(cfg, &device.device_id);
    let mut rpms = telemetry.fan_rpms.get(&device.device_id).cloned();
    if let Some(slot_limit) = physical_wireless_fan_limit(device) {
        slots.truncate(slot_limit);
        if let Some(values) = rpms.as_mut() {
            values.truncate(slot_limit);
        }
    }
    normalize_reported_manual_slots(device, &mut slots);
    let active_mode = summarize_fan_active_mode(&slots);
    let active_curve = summarize_fan_active_curve(&slots, &active_mode);
    let temperature_source = active_curve.as_ref().and_then(|curve_name| {
        cfg.fan_curves
            .iter()
            .find(|curve| curve.name == *curve_name)
            .map(|curve| curve.temp_command.clone())
    });
    let mb_sync_enabled = slots.iter().any(|slot| slot.mode == "mb_sync");

    FanStateResponse {
        device_id: device.device_id.clone(),
        update_interval_ms,
        rpms,
        slots,
        active_mode,
        active_curve,
        temperature_source,
        mb_sync_enabled,
        start_stop_supported: false,
        start_stop_enabled: false,
    }
}

fn physical_wireless_fan_limit(device: &DeviceInfo) -> Option<usize> {
    if device.wireless_channel.is_none() {
        return None;
    }

    match device.fan_count {
        Some(1..=4) => Some(device.fan_count.unwrap_or(0) as usize),
        _ => None,
    }
}

fn normalize_reported_manual_slots(device: &DeviceInfo, slots: &mut [FanSlotState]) {
    let Some(min_percent) = stable_manual_percent_floor(device) else {
        return;
    };

    for slot in slots {
        if slot.mode != "manual" {
            continue;
        }
        let Some(percent) = slot.percent else {
            continue;
        };
        if percent == 0 || percent >= min_percent {
            continue;
        }

        slot.percent = Some(min_percent);
        slot.pwm = Some(percent_to_pwm(min_percent));
    }
}

fn normalize_manual_percent_for_device(device: &DeviceInfo, percent: u8) -> u8 {
    match stable_manual_percent_floor(device) {
        Some(min_percent) if percent > 0 && percent < min_percent => min_percent,
        _ => percent,
    }
}

fn stable_manual_percent_floor(device: &DeviceInfo) -> Option<u8> {
    if device.wireless_channel.is_some()
        && matches!(device.family, DeviceFamily::SlInf)
        && device.fan_count == Some(1)
    {
        Some(30)
    } else {
        None
    }
}

fn summarize_fan_active_mode(slots: &[FanSlotState]) -> String {
    if slots.is_empty() {
        return "manual".to_string();
    }

    let unique_modes = slots
        .iter()
        .map(|slot| slot.mode.as_str())
        .collect::<HashSet<_>>();

    if unique_modes.len() == 1 {
        unique_modes
            .into_iter()
            .next()
            .unwrap_or("manual")
            .to_string()
    } else {
        "mixed".to_string()
    }
}

fn summarize_fan_active_curve(slots: &[FanSlotState], active_mode: &str) -> Option<String> {
    if active_mode != "curve" {
        return None;
    }

    let curve_names = slots
        .iter()
        .filter_map(|slot| slot.curve.as_deref())
        .collect::<HashSet<_>>();

    if curve_names.len() == 1 {
        curve_names.into_iter().next().map(str::to_string)
    } else {
        None
    }
}

fn ensure_fan_group_mut<'a>(cfg: &'a mut FanConfig, device_id: &str) -> &'a mut FanGroup {
    if let Some(index) = cfg
        .speeds
        .iter()
        .position(|group| group.device_id.as_deref() == Some(device_id))
    {
        return &mut cfg.speeds[index];
    }

    cfg.speeds.push(FanGroup {
        device_id: Some(device_id.to_string()),
        speeds: default_fan_speeds(0),
    });
    cfg.speeds.last_mut().expect("fan group exists after push")
}

fn parse_color(color: &crate::models::ColorValue) -> ApiResult<[u8; 3]> {
    if let Some(hex) = &color.hex {
        return parse_hex_color(hex);
    }

    if let Some(rgb) = &color.rgb {
        return Ok([rgb.r, rgb.g, rgb.b]);
    }

    Err(crate::errors::ApiError::BadRequest(
        "color required: use hex or rgb".to_string(),
    ))
}

fn parse_rgb_mode(input: &str) -> ApiResult<RgbMode> {
    let norm = input
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '_')
        .collect::<String>();

    let mode = match norm.as_str() {
        "off" => RgbMode::Off,
        "direct" => RgbMode::Direct,
        "static" => RgbMode::Static,
        "rainbow" => RgbMode::Rainbow,
        "rainbowmorph" => RgbMode::RainbowMorph,
        "breathing" => RgbMode::Breathing,
        "runway" => RgbMode::Runway,
        "meteor" => RgbMode::Meteor,
        "colorcycle" => RgbMode::ColorCycle,
        "staggered" => RgbMode::Staggered,
        "tide" => RgbMode::Tide,
        "mixing" => RgbMode::Mixing,
        "voice" => RgbMode::Voice,
        "door" => RgbMode::Door,
        "render" => RgbMode::Render,
        "ripple" => RgbMode::Ripple,
        "reflect" => RgbMode::Reflect,
        "tailchasing" => RgbMode::TailChasing,
        "paint" => RgbMode::Paint,
        "pingpong" => RgbMode::PingPong,
        "stack" => RgbMode::Stack,
        "covercycle" => RgbMode::CoverCycle,
        "wave" => RgbMode::Wave,
        "racing" => RgbMode::Racing,
        "lottery" => RgbMode::Lottery,
        "intertwine" => RgbMode::Intertwine,
        "meteorshower" => RgbMode::MeteorShower,
        "collide" => RgbMode::Collide,
        "electriccurrent" => RgbMode::ElectricCurrent,
        "kaleidoscope" => RgbMode::Kaleidoscope,
        "bigbang" => RgbMode::BigBang,
        "vortex" => RgbMode::Vortex,
        "pump" => RgbMode::Pump,
        "colorsmorph" => RgbMode::ColorsMorph,
        _ => {
            return Err(crate::errors::ApiError::BadRequest(format!(
                "unknown effect: {input}"
            )))
        }
    };

    Ok(mode)
}

fn parse_direction(input: &str) -> ApiResult<RgbDirection> {
    let norm = input.trim().to_lowercase();
    let dir = match norm.as_str() {
        "clockwise" | "cw" => RgbDirection::Clockwise,
        "counterclockwise" | "ccw" => RgbDirection::CounterClockwise,
        "up" => RgbDirection::Up,
        "down" => RgbDirection::Down,
        "spread" => RgbDirection::Spread,
        "gather" => RgbDirection::Gather,
        _ => {
            return Err(crate::errors::ApiError::BadRequest(format!(
                "unknown direction: {input}"
            )))
        }
    };
    Ok(dir)
}

fn parse_scope(input: &str) -> ApiResult<RgbScope> {
    let norm = input.trim().to_lowercase();
    let scope = match norm.as_str() {
        "all" => RgbScope::All,
        "top" => RgbScope::Top,
        "bottom" => RgbScope::Bottom,
        "inner" => RgbScope::Inner,
        "outer" => RgbScope::Outer,
        _ => {
            return Err(crate::errors::ApiError::BadRequest(format!(
                "unknown scope: {input}"
            )))
        }
    };
    Ok(scope)
}

fn parse_hid_driver(input: &str) -> ApiResult<HidDriver> {
    match input.trim().to_lowercase().as_str() {
        "hidapi" => Ok(HidDriver::Hidapi),
        "rusb" => Ok(HidDriver::Rusb),
        other => Err(crate::errors::ApiError::BadRequest(format!(
            "unknown hid_driver: {other}"
        ))),
    }
}

fn hid_driver_name(driver: HidDriver) -> &'static str {
    match driver {
        HidDriver::Hidapi => "hidapi",
        HidDriver::Rusb => "rusb",
    }
}

fn parse_media_type(input: &str) -> ApiResult<MediaType> {
    match input.trim().to_lowercase().as_str() {
        "image" => Ok(MediaType::Image),
        "video" => Ok(MediaType::Video),
        "color" => Ok(MediaType::Color),
        "gif" => Ok(MediaType::Gif),
        "sensor" => Ok(MediaType::Sensor),
        other => Err(crate::errors::ApiError::BadRequest(format!(
            "unknown media type: {other}"
        ))),
    }
}

fn media_type_name(media_type: MediaType) -> &'static str {
    match media_type {
        MediaType::Image => "image",
        MediaType::Video => "video",
        MediaType::Color => "color",
        MediaType::Gif => "gif",
        MediaType::Sensor => "sensor",
    }
}

fn rgb_mode_name(mode: RgbMode) -> String {
    format!("{mode:?}")
}

fn rgb_direction_name(direction: RgbDirection) -> String {
    format!("{direction:?}")
}

fn rgb_scope_name(scope: RgbScope) -> String {
    format!("{scope:?}")
}

fn normalize_orientation(orientation: f32) -> f32 {
    let normalized = (orientation % 360.0 + 360.0) % 360.0;
    let snapped = ((normalized + 45.0) / 90.0).floor() * 90.0;
    snapped % 360.0
}

fn percent_to_pwm(percent: u8) -> u8 {
    ((percent as f32 / 100.0) * 255.0).round() as u8
}

fn pwm_to_percent(pwm: u8) -> u8 {
    ((pwm as f32 / 255.0) * 100.0).round() as u8
}

fn percent_to_brightness(percent: u8) -> u8 {
    let scaled = ((percent as f32 / 100.0) * 4.0).round() as i32;
    scaled.clamp(0, 4) as u8
}

fn brightness_to_percent(level: u8) -> u8 {
    let scaled = ((level as f32 / 4.0) * 100.0).round() as i32;
    scaled.clamp(0, 100) as u8
}

fn default_fan_config() -> FanConfig {
    FanConfig {
        speeds: Vec::new(),
        update_interval_ms: 1000,
    }
}

fn default_fan_speeds(pwm: u8) -> [FanSpeed; 4] {
    [
        FanSpeed::Constant(pwm),
        FanSpeed::Constant(pwm),
        FanSpeed::Constant(pwm),
        FanSpeed::Constant(pwm),
    ]
}

fn rgb_to_hex(color: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

fn parse_hex_color(input: &str) -> ApiResult<[u8; 3]> {
    let hex = input.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return Err(crate::errors::ApiError::BadRequest(
            "hex color must be 6 digits (RRGGBB)".to_string(),
        ));
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| crate::errors::ApiError::BadRequest("invalid hex color".to_string()))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| crate::errors::ApiError::BadRequest("invalid hex color".to_string()))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| crate::errors::ApiError::BadRequest("invalid hex color".to_string()))?;

    Ok([r, g, b])
}

trait AppConfigExt {
    fn with_fans(self, fans: FanConfig) -> Self;
}

impl AppConfigExt for AppConfig {
    fn with_fans(mut self, fans: FanConfig) -> Self {
        self.fans = Some(fans);
        self
    }
}











































