use crate::app::AppState;
use crate::config::{xdg_config_home, xdg_runtime_dir};
use crate::errors::ApiResult;
use crate::models::{
    BackendAuthRuntime, BackendRuntime, ConfigDocument, DaemonRuntime, DaemonStatusResponse,
    DeviceCapability, DeviceState, DeviceView, FanConfigDocument, FanCurveDocument,
    FanCurvePointDocument, FanDeviceConfigDocument, FanManualRequest, FanSlotConfigDocument,
    FanSlotState, FanStateResponse, HealthResponse, LcdConfigDocument,
    LightingBrightnessRequest, LightingColorRequest, LightingConfigDocument,
    LightingDeviceConfigDocument, LightingEffectRequest, LightingStateResponse,
    LightingZoneConfigDocument, LightingZoneState, ProfileApplyResponse, ProfileApplySkip,
    ProfileDocument, ProfileFanDocument, ProfileLightingDocument, ProfileMetadataDocument,
    ProfileTargetsDocument, ProfileUpsertDocument, RuntimeResponse, SensorConfigDocument,
    SensorRangeDocument, SensorSourceDocument, VersionResponse,
};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use futures_util::StreamExt;
use lianli_shared::config::{AppConfig, HidDriver, LcdConfig};
use lianli_shared::fan::{FanConfig, FanCurve, FanGroup, FanSpeed, MB_SYNC_KEY};
use lianli_shared::ipc::{DeviceInfo, IpcRequest, IpcResponse, TelemetrySnapshot};
use lianli_shared::media::{MediaType, SensorDescriptor, SensorRange, SensorSourceConfig};
use lianli_shared::rgb::{
    RgbAppConfig, RgbDeviceConfig, RgbDirection, RgbEffect, RgbMode, RgbScope, RgbZoneConfig,
};
use std::collections::HashSet;
use std::path::PathBuf;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration as TokioDuration, MissedTickBehavior};

pub async fn health() -> ApiResult<Json<HealthResponse>> {
    Ok(Json(HealthResponse { status: "ok" }))
}

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
    let devices: Vec<DeviceInfo> = unwrap_response(state.daemon.send(&IpcRequest::ListDevices)?)?;
    let telemetry: TelemetrySnapshot =
        unwrap_response(state.daemon.send(&IpcRequest::GetTelemetry)?)?;

    let views = devices
        .into_iter()
        .map(|d| device_view(d, &telemetry))
        .collect();

    Ok(Json(views))
}

pub async fn get_device(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<DeviceView>> {
    let devices: Vec<DeviceInfo> = unwrap_response(state.daemon.send(&IpcRequest::ListDevices)?)?;
    let telemetry: TelemetrySnapshot =
        unwrap_response(state.daemon.send(&IpcRequest::GetTelemetry)?)?;

    let device = devices
        .into_iter()
        .find(|d| d.device_id == id)
        .ok_or_else(|| crate::errors::ApiError::NotFound(format!("unknown device id: {id}")))?;

    Ok(Json(device_view(device, &telemetry)))
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
    require_device(&state, &id)?;
    let cfg = get_config(&state)?;
    let rgb = cfg.rgb.unwrap_or_default();
    let zones = build_lighting_zones(&rgb, &id);
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
    require_device(&state, &id)?;
    let zone = body.zone.unwrap_or(0);
    let color = parse_color(&body.color)?;

    let cfg = get_config(&state)?;
    let mut rgb = cfg.rgb.unwrap_or_default();
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
    let zones = build_lighting_zones(&rgb, &id);
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
    require_device(&state, &id)?;
    let zone = body.zone.unwrap_or(0);
    let mode = parse_rgb_mode(&body.effect)?;

    let cfg = get_config(&state)?;
    let mut rgb = cfg.rgb.unwrap_or_default();
    let mut effect = effect_for_zone(&rgb, &id, zone);
    effect.mode = mode;

    if let Some(speed) = body.speed {
        if speed > 4 {
            return Err(crate::errors::ApiError::BadRequest(
                "speed must be 0-4".to_string(),
            ));
        }
        effect.speed = speed;
    }

    if let Some(brightness) = body.brightness {
        if brightness > 100 {
            return Err(crate::errors::ApiError::BadRequest(
                "brightness percent must be 0-100".to_string(),
            ));
        }
        effect.brightness = percent_to_brightness(brightness);
    }

    if let Some(color) = body.color {
        effect.colors = vec![parse_color(&color)?];
    }

    if let Some(direction) = body.direction {
        effect.direction = parse_direction(&direction)?;
    }

    if let Some(scope) = body.scope {
        effect.scope = parse_scope(&scope)?;
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
    let zones = build_lighting_zones(&rgb, &id);
    Ok(Json(LightingStateResponse {
        device_id: id,
        zones,
    }))
}

pub async fn set_lighting_brightness(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<LightingBrightnessRequest>,
) -> ApiResult<Json<LightingStateResponse>> {
    require_device(&state, &id)?;
    let zone = body.zone.unwrap_or(0);
    if body.percent > 100 {
        return Err(crate::errors::ApiError::BadRequest(
            "brightness percent must be 0-100".to_string(),
        ));
    }

    let cfg = get_config(&state)?;
    let mut rgb = cfg.rgb.unwrap_or_default();
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

    let zones = build_lighting_zones(&rgb, &id);
    Ok(Json(LightingStateResponse {
        device_id: id,
        zones,
    }))
}

pub async fn get_fan_state(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<FanStateResponse>> {
    require_device(&state, &id)?;
    let cfg = get_config(&state)?;
    let telemetry: TelemetrySnapshot =
        unwrap_response(state.daemon.send(&IpcRequest::GetTelemetry)?)?;
    let (slots, update_interval_ms) = fan_slots_from_config(&cfg, &id);
    let rpms = telemetry.fan_rpms.get(&id).cloned();

    Ok(Json(FanStateResponse {
        device_id: id,
        update_interval_ms,
        rpms,
        slots,
    }))
}

pub async fn set_fan_manual(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<FanManualRequest>,
) -> ApiResult<Json<FanStateResponse>> {
    require_device(&state, &id)?;
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
    let pwm = percent_to_pwm(body.percent);

    let group = match fan_cfg
        .speeds
        .iter_mut()
        .find(|g| g.device_id.as_deref() == Some(&id))
    {
        Some(g) => g,
        None => {
            fan_cfg.speeds.push(FanGroup {
                device_id: Some(id.clone()),
                speeds: default_fan_speeds(0),
            });
            fan_cfg.speeds.last_mut().unwrap()
        }
    };

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
            "percent": body.percent,
        }),
    );
    let (slots, update_interval_ms) = fan_slots_from_config(&cfg.with_fans(fan_cfg), &id);

    Ok(Json(FanStateResponse {
        device_id: id,
        update_interval_ms,
        rpms: None,
        slots,
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

fn device_view(device: DeviceInfo, telemetry: &TelemetrySnapshot) -> DeviceView {
    let fan_rpms = telemetry.fan_rpms.get(&device.device_id).cloned();
    let coolant_temp = telemetry.coolant_temps.get(&device.device_id).cloned();
    let online = true;

    DeviceView {
        id: device.device_id.clone(),
        name: device.name,
        family: format!("{:?}", device.family),
        online,
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
    LightingConfigDocument {
        enabled: cfg.enabled,
        openrgb_server: cfg.openrgb_server,
        openrgb_port: cfg.openrgb_port,
        devices: cfg
            .devices
            .into_iter()
            .map(|device| LightingDeviceConfigDocument {
                device_id: device.device_id,
                motherboard_sync: device.mb_rgb_sync,
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
        devices,
    } = doc;

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
                    if zone.speed > 4 {
                        return Err(crate::errors::ApiError::BadRequest(
                            "lighting speed must be 0-4".to_string(),
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
                        },
                        swap_lr: zone.swap_left_right,
                        swap_tb: zone.swap_top_bottom,
                    })
                })
                .collect::<ApiResult<Vec<_>>>()?;

            Ok(RgbDeviceConfig {
                device_id,
                mb_rgb_sync: device.motherboard_sync,
                zones,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;

    let rgb = RgbAppConfig {
        enabled,
        openrgb_server,
        openrgb_port,
        devices,
    };

    if rgb.enabled && !rgb.openrgb_server && rgb.openrgb_port == 6743 && rgb.devices.is_empty() {
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
        for device in &rgb.devices {
            if device.device_id.trim().is_empty() {
                return Err(crate::errors::ApiError::BadRequest(
                    "lighting device_id must not be empty".to_string(),
                ));
            }

            for zone in &device.zones {
                if zone.effect.speed > 4 {
                    return Err(crate::errors::ApiError::BadRequest(
                        "lighting speed must be 0-4".to_string(),
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
        if speed > 4 {
            return Err(crate::errors::ApiError::BadRequest(
                "profile lighting speed must be 0-4".to_string(),
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

        let zone_indexes = existing_or_default_zone_indexes(rgb, &device.device_id);
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

fn existing_or_default_zone_indexes(cfg: &RgbAppConfig, device_id: &str) -> Vec<u8> {
    let Some(device) = cfg.devices.iter().find(|device| device.device_id == device_id) else {
        return vec![0];
    };

    if device.zones.is_empty() {
        vec![0]
    } else {
        device.zones.iter().map(|zone| zone.zone_index).collect()
    }
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

fn build_lighting_zones(cfg: &RgbAppConfig, device_id: &str) -> Vec<LightingZoneState> {
    let Some(dev) = cfg.devices.iter().find(|d| d.device_id == device_id) else {
        return Vec::new();
    };

    dev.zones
        .iter()
        .map(|zone| LightingZoneState {
            zone: zone.zone_index,
            effect: rgb_mode_name(zone.effect.mode),
            colors: zone
                .effect
                .colors
                .iter()
                .map(|c| rgb_to_hex(*c))
                .collect(),
            speed: zone.effect.speed,
            brightness_percent: brightness_to_percent(zone.effect.brightness),
            direction: rgb_direction_name(zone.effect.direction),
            scope: rgb_scope_name(zone.effect.scope),
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
    }
}

fn upsert_zone_effect(cfg: &mut RgbAppConfig, device_id: &str, zone: u8, effect: RgbEffect) {
    let dev = match cfg.devices.iter_mut().find(|d| d.device_id == device_id) {
        Some(d) => d,
        None => {
            cfg.devices.push(RgbDeviceConfig {
                device_id: device_id.to_string(),
                mb_rgb_sync: false,
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
