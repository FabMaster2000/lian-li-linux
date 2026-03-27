use crate::app::AppState;
use crate::config::{AppConfig as BackendConfig, AuthConfig, AuthMode};
use crate::daemon::DaemonClient;
use crate::events::{EventHub, WebEvent};
use crate::models::{
    ConfigDocument, FanConfigDocument, FanDeviceConfigDocument, FanSlotConfigDocument,
    LcdConfigDocument, LightingConfigDocument, LightingDeviceConfigDocument,
    LightingLedZoneConfigDocument, LightingZoneConfigDocument, ProfileApplyResponse,
    ProfileDocument, ProfileFanDocument,
    ProfileLightingDocument, ProfileTargetsDocument, ProfileUpsertDocument,
    SensorConfigDocument, SensorRangeDocument, SensorSourceDocument,
};
use crate::routes;
use crate::storage::ProfileStore;
use axum::body::Body;
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::{Method, Request, StatusCode};
use axum::response::Response;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use hyper::body::to_bytes;
use serde_json::json;
use lianli_shared::device_id::DeviceFamily;
use lianli_shared::config::{AppConfig as DaemonConfig, HidDriver, LcdConfig};
use lianli_shared::fan::{FanConfig, FanGroup, FanSpeed};
use lianli_shared::ipc::{
    DeviceInfo, IpcRequest, IpcResponse, TelemetrySnapshot, WirelessBindingState,
};
use lianli_shared::media::MediaType;
use lianli_shared::rgb::{
    RgbAppConfig, RgbDeviceCapabilities, RgbDeviceConfig, RgbDirection, RgbEffect,
    RgbFanLedZoneConfig, RgbLedZoneConfig, RgbMode, RgbScope, RgbZoneConfig,
};
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use tempfile::TempDir;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tower::ServiceExt;

struct MockDaemon {
    socket_path: PathBuf,
    requests: Arc<Mutex<Vec<IpcRequest>>>,
    handle: JoinHandle<()>,
    _tempdir: TempDir,
}

impl MockDaemon {
    fn new<F>(expected_connections: usize, responder: F) -> Self
    where
        F: Fn(IpcRequest) -> IpcResponse + Send + Sync + 'static,
    {
        let tempdir = tempfile::tempdir().expect("create temp dir");
        let socket_path = tempdir.path().join("daemon.sock");
        let listener = UnixListener::bind(&socket_path).expect("bind unix socket");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_for_thread = Arc::clone(&requests);
        let responder = Arc::new(responder);
        let responder_for_thread = Arc::clone(&responder);

        let handle = thread::spawn(move || {
            for _ in 0..expected_connections {
                let (mut stream, _) = listener.accept().expect("accept connection");
                let mut line = String::new();
                let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
                loop {
                    line.clear();
                    let bytes = reader.read_line(&mut line).expect("read request");
                    assert!(bytes > 0, "expected request line");
                    if !line.trim().is_empty() {
                        break;
                    }
                }

                let request: IpcRequest =
                    serde_json::from_str(line.trim()).expect("parse ipc request");
                requests_for_thread
                    .lock()
                    .expect("lock requests")
                    .push(request.clone());
                let response = responder_for_thread(request);
                let json = serde_json::to_string(&response).expect("serialize response");
                stream.write_all(json.as_bytes()).expect("write response");
                stream.write_all(b"\n").expect("write response newline");
                stream.flush().expect("flush response");
            }
        });

        Self {
            socket_path,
            requests,
            handle,
            _tempdir: tempdir,
        }
    }

    fn app(&self) -> axum::Router {
        self.app_with_events(EventHub::new())
    }

    fn app_with_events(&self, events: EventHub) -> axum::Router {
        let profile_store_path = self._tempdir.path().join("profiles.json");
        self.app_with_config(
            BackendConfig {
                host: IpAddr::V4(Ipv4Addr::LOCALHOST),
                port: 9000,
                socket_path: self.socket_path.clone(),
                log_level: "info".to_string(),
                backend_config_path: self._tempdir.path().join("backend.json"),
                daemon_config_path: self._tempdir.path().join("config.json"),
                profile_store_path,
                auth: AuthConfig {
                    mode: AuthMode::None,
                    basic_username: None,
                    basic_password: None,
                    bearer_token: None,
                    proxy_header: None,
                },
            },
            events,
        )
    }

    fn app_with_config(&self, config: BackendConfig, events: EventHub) -> axum::Router {
        let profile_store_path = config.profile_store_path.clone();
        let state = AppState {
            daemon: DaemonClient::new(config.socket_path.clone()),
            config,
            profiles: ProfileStore::new(profile_store_path),
            events,
        };

        routes::router(state)
    }

    fn join(self) -> Vec<IpcRequest> {
        self.handle.join().expect("join mock daemon");
        Arc::try_unwrap(self.requests)
            .expect("unwrap requests")
            .into_inner()
            .expect("unlock requests")
    }
}

fn manual_speeds(pwm: u8) -> [FanSpeed; 4] {
    [
        FanSpeed::Constant(pwm),
        FanSpeed::Constant(pwm),
        FanSpeed::Constant(pwm),
        FanSpeed::Constant(pwm),
    ]
}

fn sample_daemon_config() -> DaemonConfig {
    DaemonConfig {
        default_fps: 24.0,
        hid_driver: HidDriver::Rusb,
        lcds: vec![LcdConfig {
            index: None,
            serial: Some("LCD123".to_string()),
            media_type: MediaType::Color,
            path: None,
            fps: Some(12.0),
            rgb: Some([255, 255, 255]),
            orientation: 90.0,
            sensor: None,
        }],
        fan_curves: Vec::new(),
        fans: Some(FanConfig {
            speeds: vec![FanGroup {
                device_id: Some("wireless:test".to_string()),
                speeds: manual_speeds(107),
            }],
            update_interval_ms: 900,
        }),
        rgb: Some(RgbAppConfig {
            enabled: true,
            openrgb_server: false,
            openrgb_port: 6743,
            global_led_zones: vec![RgbLedZoneConfig {
                zone_index: 0,
                led_indexes: vec![0, 1, 2],
            }],
            fan_led_zones: Vec::new(),
            effect_route: Vec::new(),
            devices: vec![RgbDeviceConfig {
                device_id: "wireless:test".to_string(),
                mb_rgb_sync: false,
                zones: vec![RgbZoneConfig {
                    zone_index: 0,
                    effect: RgbEffect {
                        mode: RgbMode::Static,
                        colors: vec![[0x11, 0x22, 0x33]],
                        speed: 2,
                        brightness: 3,
                        direction: RgbDirection::Clockwise,
                        scope: RgbScope::All,
                        smoothness_ms: 0,
                    },
                    swap_lr: false,
                    swap_tb: false,
                }],
                led_zones: vec![RgbLedZoneConfig {
                    zone_index: 0,
                    led_indexes: vec![0, 1, 2],
                }],
            }],
        }),
    }
}

fn sample_config_document() -> ConfigDocument {
    ConfigDocument {
        default_fps: 30.0,
        hid_driver: "rusb".to_string(),
        lighting: LightingConfigDocument {
            enabled: true,
            openrgb_server: false,
            openrgb_port: 6743,
            global_led_zones: vec![LightingLedZoneConfigDocument {
                zone: 0,
                led_indexes: vec![0, 1, 2],
            }],
            fan_led_zones: Vec::new(),
            effect_route: Vec::new(),
            devices: vec![LightingDeviceConfigDocument {
                device_id: "wireless:test".to_string(),
                motherboard_sync: false,
                zones: vec![LightingZoneConfigDocument {
                    zone: 0,
                    effect: "Static".to_string(),
                    colors: vec!["#abcdef".to_string()],
                    speed: 2,
                    brightness_percent: 75,
                    direction: "Clockwise".to_string(),
                    scope: "All".to_string(),
                    swap_left_right: false,
                    swap_top_bottom: false,
                    smoothness_ms: 0,
                }],
                led_zones: vec![LightingLedZoneConfigDocument {
                    zone: 0,
                    led_indexes: vec![0, 1, 2],
                }],
            }],
        },
        fans: FanConfigDocument {
            update_interval_ms: 1000,
            curves: Vec::new(),
            devices: vec![FanDeviceConfigDocument {
                device_id: Some("wireless:test".to_string()),
                slots: vec![
                    FanSlotConfigDocument {
                        slot: 1,
                        mode: "manual".to_string(),
                        percent: Some(42),
                        curve: None,
                    },
                    FanSlotConfigDocument {
                        slot: 2,
                        mode: "manual".to_string(),
                        percent: Some(42),
                        curve: None,
                    },
                    FanSlotConfigDocument {
                        slot: 3,
                        mode: "manual".to_string(),
                        percent: Some(42),
                        curve: None,
                    },
                    FanSlotConfigDocument {
                        slot: 4,
                        mode: "manual".to_string(),
                        percent: Some(42),
                        curve: None,
                    },
                ],
            }],
        },
        lcds: vec![LcdConfigDocument {
            device_id: None,
            index: None,
            serial: Some("LCD123".to_string()),
            media: "color".to_string(),
            path: None,
            fps: Some(15.0),
            color: Some("#123456".to_string()),
            orientation: 91.0,
            sensor: None,
        }],
    }
}

fn sample_profile_request() -> ProfileUpsertDocument {
    ProfileUpsertDocument {
        id: "night-mode".to_string(),
        name: "Night Mode".to_string(),
        description: Some("Dim lighting and reduce fan speed".to_string()),
        targets: ProfileTargetsDocument {
            mode: "all".to_string(),
            device_ids: Vec::new(),
        },
        lighting: Some(ProfileLightingDocument {
            enabled: true,
            color: Some("#223366".to_string()),
            effect: Some("Static".to_string()),
            brightness_percent: Some(15),
            speed: None,
            direction: None,
            scope: None,
        }),
        fans: Some(ProfileFanDocument {
            enabled: true,
            mode: "manual".to_string(),
            percent: Some(25),
        }),
    }
}

fn device_info(device_id: &str, name: &str, has_rgb: bool, has_fan: bool) -> DeviceInfo {
    DeviceInfo {
        device_id: device_id.to_string(),
        family: DeviceFamily::SlInf,
        name: name.to_string(),
        serial: None,
        wireless_channel: None,
        wireless_missed_polls: None,
        has_lcd: false,
        has_fan,
        has_pump: false,
        has_rgb,
        fan_count: Some(4),
        per_fan_control: Some(false),
        mb_sync_support: false,
        rgb_zone_count: Some(1),
        screen_width: None,
        screen_height: None,
        wireless_master_mac: None,
        wireless_binding_state: None,
    }
}

fn json_request(method: Method, uri: &str, body: impl serde::Serialize) -> Request<Body> {
    let body = serde_json::to_vec(&body).expect("serialize request body");
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .expect("build request")
}

fn empty_request(method: Method, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .expect("build request")
}

fn request_with_header(method: Method, uri: &str, header_name: &str, header_value: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header_name, header_value)
        .body(Body::empty())
        .expect("build request")
}

async fn read_json<T: serde::de::DeserializeOwned>(response: Response) -> T {
    let body = to_bytes(response.into_body()).await.expect("read body");
    serde_json::from_slice(&body).expect("parse json body")
}

async fn assert_api_error(
    response: Response,
    status: StatusCode,
    code: &str,
    message: &str,
) {
    assert_eq!(response.status(), status);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["error"]["code"], code);
    assert_eq!(body["error"]["message"], message);
}

async fn spawn_test_server(
    app: axum::Router,
) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");
    listener
        .set_nonblocking(true)
        .expect("set nonblocking listener");

    let server = axum::Server::from_tcp(listener)
        .expect("server from tcp")
        .serve(app.into_make_service());
    let handle = tokio::spawn(async move {
        server.await.expect("run test server");
    });

    (addr, handle)
}

async fn receive_websocket_text<S>(
    socket: &mut tokio_tungstenite::WebSocketStream<S>,
) -> String
where
    tokio_tungstenite::WebSocketStream<S>:
        futures_util::Stream<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>>
            + Unpin,
{
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let message = socket
                .next()
                .await
                .expect("websocket message")
                .expect("websocket frame");

            match message {
                WsMessage::Text(payload) => return payload,
                WsMessage::Ping(_) | WsMessage::Pong(_) | WsMessage::Binary(_) => continue,
                WsMessage::Close(frame) => panic!("unexpected websocket close: {frame:?}"),
                WsMessage::Frame(_) => continue,
            }
        }
    })
    .await
    .expect("websocket receive timeout")
}

fn encoded_device_id(device_id: &str) -> String {
    device_id.replace(':', "%3A")
}

fn sample_device_config(device_id: &str) -> DaemonConfig {
    let mut cfg = sample_daemon_config();
    cfg.fans = Some(FanConfig {
        speeds: vec![FanGroup {
            device_id: Some(device_id.to_string()),
            speeds: manual_speeds(107),
        }],
        update_interval_ms: 900,
    });

    if let Some(rgb) = cfg.rgb.as_mut() {
        rgb.devices = vec![RgbDeviceConfig {
            device_id: device_id.to_string(),
            mb_rgb_sync: false,
            zones: vec![RgbZoneConfig {
                zone_index: 0,
                effect: RgbEffect {
                    mode: RgbMode::Static,
                    colors: vec![[0x11, 0x22, 0x33]],
                    speed: 2,
                    brightness: 3,
                    direction: RgbDirection::Clockwise,
                    scope: RgbScope::All,
                    smoothness_ms: 0,
                },
                swap_lr: false,
                swap_tb: false,
            }],
            led_zones: vec![RgbLedZoneConfig {
                zone_index: 0,
                led_indexes: vec![0, 1, 2],
            }],
        }];
    }

    cfg
}

fn sample_telemetry(device_id: &str) -> TelemetrySnapshot {
    let mut telemetry = TelemetrySnapshot::default();
    telemetry
        .fan_rpms
        .insert(device_id.to_string(), vec![910, 920, 930, 940]);
    telemetry
        .coolant_temps
        .insert(device_id.to_string(), 31.5);
    telemetry.streaming_active = true;
    telemetry
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, "/api/health"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body, json!({ "status": "ok" }));

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn version_endpoint_returns_package_version() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, "/api/version"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body, json!({ "version": env!("CARGO_PKG_VERSION") }));

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn health_endpoint_stays_public_when_auth_is_enabled() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let app = mock.app_with_config(
        BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9000,
            socket_path: mock.socket_path.clone(),
            log_level: "info".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: mock._tempdir.path().join("config.json"),
            profile_store_path: mock._tempdir.path().join("profiles.json"),
            auth: AuthConfig {
                mode: AuthMode::Basic,
                basic_username: Some("admin".to_string()),
                basic_password: Some("secret".to_string()),
                bearer_token: None,
                proxy_header: None,
            },
        },
        EventHub::new(),
    );

    let response = app
        .oneshot(empty_request(Method::GET, "/api/health"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn protected_endpoint_requires_basic_auth_when_enabled() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let app = mock.app_with_config(
        BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9000,
            socket_path: mock.socket_path.clone(),
            log_level: "info".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: mock._tempdir.path().join("config.json"),
            profile_store_path: mock._tempdir.path().join("profiles.json"),
            auth: AuthConfig {
                mode: AuthMode::Basic,
                basic_username: Some("admin".to_string()),
                basic_password: Some("secret".to_string()),
                bearer_token: None,
                proxy_header: None,
            },
        },
        EventHub::new(),
    );

    let response = app
        .oneshot(empty_request(Method::GET, "/api/devices"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response
            .headers()
            .get(WWW_AUTHENTICATE)
            .and_then(|value| value.to_str().ok()),
        Some(r#"Basic realm="lianli-backend""#)
    );
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["error"]["code"], "UNAUTHORIZED");

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn protected_endpoint_accepts_basic_auth_when_credentials_match() {
    let mock = MockDaemon::new(2, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(vec![device_info(
            "wireless:test",
            "Auth Device",
            true,
            true,
        )]),
        IpcRequest::GetTelemetry => IpcResponse::ok(sample_telemetry("wireless:test")),
        other => panic!("unexpected request: {other:?}"),
    });
    let app = mock.app_with_config(
        BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9000,
            socket_path: mock.socket_path.clone(),
            log_level: "info".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: mock._tempdir.path().join("config.json"),
            profile_store_path: mock._tempdir.path().join("profiles.json"),
            auth: AuthConfig {
                mode: AuthMode::Basic,
                basic_username: Some("admin".to_string()),
                basic_password: Some("secret".to_string()),
                bearer_token: None,
                proxy_header: None,
            },
        },
        EventHub::new(),
    );
    let token = STANDARD.encode("admin:secret");

    let response = app
        .oneshot(request_with_header(
            Method::GET,
            "/api/devices",
            AUTHORIZATION.as_str(),
            &format!("Basic {token}"),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);

    let requests = mock.join();
    assert_eq!(requests.len(), 2);
}

#[tokio::test]
async fn protected_endpoint_rejects_basic_auth_with_wrong_credentials() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let app = mock.app_with_config(
        BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9000,
            socket_path: mock.socket_path.clone(),
            log_level: "info".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: mock._tempdir.path().join("config.json"),
            profile_store_path: mock._tempdir.path().join("profiles.json"),
            auth: AuthConfig {
                mode: AuthMode::Basic,
                basic_username: Some("admin".to_string()),
                basic_password: Some("secret".to_string()),
                bearer_token: None,
                proxy_header: None,
            },
        },
        EventHub::new(),
    );
    let token = STANDARD.encode("admin:wrong");

    let response = app
        .oneshot(request_with_header(
            Method::GET,
            "/api/devices",
            AUTHORIZATION.as_str(),
            &format!("Basic {token}"),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response
            .headers()
            .get(WWW_AUTHENTICATE)
            .and_then(|value| value.to_str().ok()),
        Some(r#"Basic realm="lianli-backend""#)
    );

    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["error"]["code"], "UNAUTHORIZED");
    assert_eq!(body["error"]["message"], "unauthorized: invalid username or password");
    assert_eq!(body["error"]["details"]["source"], "auth");

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn protected_endpoint_accepts_bearer_token_when_configured() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let app = mock.app_with_config(
        BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9000,
            socket_path: mock.socket_path.clone(),
            log_level: "info".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: mock._tempdir.path().join("config.json"),
            profile_store_path: mock._tempdir.path().join("profiles.json"),
            auth: AuthConfig {
                mode: AuthMode::Bearer,
                basic_username: None,
                basic_password: None,
                bearer_token: Some("test-token".to_string()),
                proxy_header: None,
            },
        },
        EventHub::new(),
    );

    let response = app
        .oneshot(request_with_header(
            Method::GET,
            "/api/runtime",
            AUTHORIZATION.as_str(),
            "Bearer test-token",
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn protected_endpoint_rejects_bearer_token_with_wrong_token() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let app = mock.app_with_config(
        BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9000,
            socket_path: mock.socket_path.clone(),
            log_level: "info".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: mock._tempdir.path().join("config.json"),
            profile_store_path: mock._tempdir.path().join("profiles.json"),
            auth: AuthConfig {
                mode: AuthMode::Bearer,
                basic_username: None,
                basic_password: None,
                bearer_token: Some("test-token".to_string()),
                proxy_header: None,
            },
        },
        EventHub::new(),
    );

    let response = app
        .oneshot(request_with_header(
            Method::GET,
            "/api/runtime",
            AUTHORIZATION.as_str(),
            "Bearer wrong-token",
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response
            .headers()
            .get(WWW_AUTHENTICATE)
            .and_then(|value| value.to_str().ok()),
        Some(r#"Bearer realm="lianli-backend""#)
    );

    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["error"]["code"], "UNAUTHORIZED");
    assert_eq!(body["error"]["message"], "unauthorized: invalid bearer token");
    assert_eq!(body["error"]["details"]["source"], "auth");

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn protected_endpoint_accepts_reverse_proxy_header_when_configured() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let app = mock.app_with_config(
        BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9000,
            socket_path: mock.socket_path.clone(),
            log_level: "info".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: mock._tempdir.path().join("config.json"),
            profile_store_path: mock._tempdir.path().join("profiles.json"),
            auth: AuthConfig {
                mode: AuthMode::ReverseProxy,
                basic_username: None,
                basic_password: None,
                bearer_token: None,
                proxy_header: Some("x-forwarded-user".to_string()),
            },
        },
        EventHub::new(),
    );

    let response = app
        .oneshot(request_with_header(
            Method::GET,
            "/api/runtime",
            "x-forwarded-user",
            "fabian",
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn protected_endpoint_rejects_reverse_proxy_auth_without_required_header() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let app = mock.app_with_config(
        BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9000,
            socket_path: mock.socket_path.clone(),
            log_level: "info".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: mock._tempdir.path().join("config.json"),
            profile_store_path: mock._tempdir.path().join("profiles.json"),
            auth: AuthConfig {
                mode: AuthMode::ReverseProxy,
                basic_username: None,
                basic_password: None,
                bearer_token: None,
                proxy_header: Some("x-forwarded-user".to_string()),
            },
        },
        EventHub::new(),
    );

    let response = app
        .oneshot(empty_request(Method::GET, "/api/runtime"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(response.headers().get(WWW_AUTHENTICATE).is_none());

    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["error"]["code"], "UNAUTHORIZED");
    assert_eq!(
        body["error"]["message"],
        "unauthorized: missing reverse proxy auth header: x-forwarded-user"
    );
    assert_eq!(body["error"]["details"]["source"], "auth");

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn websocket_route_is_protected_by_auth_middleware() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let app = mock.app_with_config(
        BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9000,
            socket_path: mock.socket_path.clone(),
            log_level: "info".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: mock._tempdir.path().join("config.json"),
            profile_store_path: mock._tempdir.path().join("profiles.json"),
            auth: AuthConfig {
                mode: AuthMode::Basic,
                basic_username: Some("admin".to_string()),
                basic_password: Some("secret".to_string()),
                bearer_token: None,
                proxy_header: None,
            },
        },
        EventHub::new(),
    );

    let response = app
        .oneshot(empty_request(Method::GET, "/api/ws"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn runtime_endpoint_returns_backend_and_daemon_paths() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, "/api/runtime"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["backend"]["host"], "127.0.0.1");
    assert_eq!(body["backend"]["port"], 9000);
    assert_eq!(body["backend"]["log_level"], "info");
    assert_eq!(
        body["backend"]["config_path"],
        json!(mock._tempdir.path().join("backend.json").display().to_string())
    );
    assert_eq!(body["backend"]["auth"]["enabled"], false);
    assert_eq!(body["backend"]["auth"]["mode"], "none");
    assert_eq!(body["backend"]["auth"]["reload_requires_restart"], true);
    assert_eq!(body["backend"]["auth"]["basic_username_configured"], false);
    assert_eq!(body["backend"]["auth"]["basic_password_configured"], false);
    assert_eq!(body["backend"]["auth"]["token_configured"], false);
    assert_eq!(
        body["daemon"]["socket_path"],
        json!(mock.socket_path.display().to_string())
    );
    assert_eq!(
        body["daemon"]["config_path"],
        json!(mock._tempdir.path().join("config.json").display().to_string())
    );
    assert!(!body["daemon"]["xdg_runtime_dir"]
        .as_str()
        .expect("xdg runtime")
        .is_empty());
    assert!(!body["daemon"]["xdg_config_home"]
        .as_str()
        .expect("xdg config")
        .is_empty());

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn devices_endpoint_returns_device_list_with_telemetry() {
    let device_id = "wireless:test:device";
    let mut device = device_info(device_id, "Sim Device", true, true);
    device.wireless_channel = Some(8);
    let devices = vec![device];
    let telemetry = sample_telemetry(device_id);
    let mock = MockDaemon::new(2, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetTelemetry => IpcResponse::ok(telemetry.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, "/api/devices"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    let items = body.as_array().expect("device array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], device_id);
    assert_eq!(items[0]["display_name"], "Sim Device 4-fan cluster [test:device]");
    assert_eq!(items[0]["physical_role"], "4-fan cluster");
    assert_eq!(items[0]["ui_order"], 0);
    assert_eq!(items[0]["controller"]["id"], "wireless:mesh");
    assert_eq!(items[0]["controller"]["label"], "Wireless dongle");
    assert_eq!(items[0]["wireless"]["channel"], json!(8));
    assert_eq!(items[0]["health"]["level"], "healthy");
    assert_eq!(items[0]["state"]["fan_rpms"], json!([910, 920, 930, 940]));
    assert_eq!(items[0]["state"]["coolant_temp"], json!(31.5));
    assert_eq!(items[0]["state"]["streaming_active"], json!(true));

    let requests = mock.join();
    assert_eq!(requests.len(), 2);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
    assert!(matches!(requests[1], IpcRequest::GetTelemetry));
}

#[tokio::test]
async fn devices_endpoint_marks_stale_wireless_devices_offline() {
    let device_id = "wireless:test:device";
    let mut device = device_info(device_id, "Sim Device", true, true);
    device.wireless_channel = Some(8);
    // Beyond the grace window (WIRELESS_ONLINE_GRACE_MISSED_POLLS = 2)
    device.wireless_missed_polls = Some(3);
    let devices = vec![device];
    let telemetry = sample_telemetry(device_id);
    let mock = MockDaemon::new(2, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetTelemetry => IpcResponse::ok(telemetry.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, "/api/devices"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    let items = body.as_array().expect("device array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], device_id);
    assert_eq!(items[0]["online"], json!(false));
    assert_eq!(items[0]["health"]["level"], "offline");
    assert_eq!(items[0]["health"]["summary"], "Wireless device not seen in the latest discovery poll");
    assert_eq!(items[0]["current_mode_summary"], "Wireless device offline");
    assert_eq!(items[0]["state"]["fan_rpms"], serde_json::Value::Null);
}

#[tokio::test]
async fn devices_endpoint_keeps_wireless_device_online_during_grace_window() {
    let device_id = "wireless:test:device";
    let mut device = device_info(device_id, "Sim Device", true, true);
    device.wireless_channel = Some(8);
    device.fan_count = Some(4);
    // Within grace window — device should remain online with RPMs visible
    device.wireless_missed_polls = Some(2);
    let devices = vec![device];
    let telemetry = sample_telemetry(device_id);
    let mock = MockDaemon::new(2, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetTelemetry => IpcResponse::ok(telemetry.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, "/api/devices"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    let items = body.as_array().expect("device array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], device_id);
    assert_eq!(items[0]["online"], json!(true));
    assert_eq!(items[0]["health"]["level"], "healthy");
    assert_ne!(items[0]["state"]["fan_rpms"], serde_json::Value::Null);
    let rpms = items[0]["state"]["fan_rpms"].as_array().expect("fan_rpms array");
    assert_eq!(rpms.len(), 4);
}
#[tokio::test]
async fn devices_endpoint_returns_bad_gateway_when_daemon_transport_fails() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let missing_socket_path = mock._tempdir.path().join("missing-daemon.sock");
    let app = mock.app_with_config(
        BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9000,
            socket_path: missing_socket_path,
            log_level: "info".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: mock._tempdir.path().join("config.json"),
            profile_store_path: mock._tempdir.path().join("profiles.json"),
            auth: AuthConfig {
                mode: AuthMode::None,
                basic_username: None,
                basic_password: None,
                bearer_token: None,
                proxy_header: None,
            },
        },
        EventHub::new(),
    );

    let response = app
        .oneshot(empty_request(Method::GET, "/api/devices"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["error"]["code"], "DAEMON_ERROR");
    assert_eq!(body["error"]["details"]["source"], "daemon_transport");
    assert!(
        body["error"]["message"]
            .as_str()
            .expect("daemon transport message")
            .starts_with("daemon error: io error:")
    );

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn devices_endpoint_returns_bad_gateway_when_daemon_replies_with_error() {
    let mock = MockDaemon::new(1, |request| match request {
        IpcRequest::ListDevices => IpcResponse::Error {
            message: "device inventory unavailable".to_string(),
        },
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, "/api/devices"))
        .await
        .expect("send request");

    assert_api_error(
        response,
        StatusCode::BAD_GATEWAY,
        "DAEMON_ERROR",
        "daemon error: device inventory unavailable",
    )
    .await;

    let requests = mock.join();
    assert_eq!(requests.len(), 1);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
}

#[tokio::test]
async fn get_device_supports_url_encoded_ids_with_colons() {
    let device_id = "wireless:test:device";
    let path = format!("/api/devices/{}", encoded_device_id(device_id));
    let devices = vec![device_info(device_id, "Sim Device", true, true)];
    let telemetry = sample_telemetry(device_id);
    let mock = MockDaemon::new(2, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetTelemetry => IpcResponse::ok(telemetry.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, &path))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["id"], device_id);
    assert_eq!(body["name"], "Sim Device");

    let requests = mock.join();
    assert_eq!(requests.len(), 2);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
    assert!(matches!(requests[1], IpcRequest::GetTelemetry));
}

#[tokio::test]
async fn update_device_presentation_persists_labels_and_ordering() {
    let device_id = "wireless:test:device";
    let path = format!("/api/devices/{}/presentation", encoded_device_id(device_id));
    let mut device = device_info(device_id, "Sim Device", true, true);
    device.wireless_channel = Some(8);
    let devices = vec![device];
    let telemetry = sample_telemetry(device_id);
    let mock = MockDaemon::new(5, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetTelemetry => IpcResponse::ok(telemetry.clone()),
        other => panic!("unexpected request: {other:?}"),
    });
    let app = mock.app();

    let update_response = app
        .clone()
        .oneshot(json_request(
            Method::PUT,
            &path,
            serde_json::json!({
                "display_name": "Desk Cluster",
                "ui_order": 25,
                "physical_role": "Rear intake cluster",
                "controller_label": "Desk wireless",
                "cluster_label": "Desk Cluster",
            }),
        ))
        .await
        .expect("send update request");

    assert_eq!(update_response.status(), StatusCode::OK);
    let updated_body: serde_json::Value = read_json(update_response).await;
    assert_eq!(updated_body["display_name"], "Desk Cluster");
    assert_eq!(updated_body["ui_order"], 25);
    assert_eq!(updated_body["physical_role"], "Rear intake cluster");
    assert_eq!(updated_body["controller"]["label"], "Desk wireless");
    assert_eq!(updated_body["wireless"]["group_label"], "Desk Cluster");

    let list_response = app
        .oneshot(empty_request(Method::GET, "/api/devices"))
        .await
        .expect("send list request");
    assert_eq!(list_response.status(), StatusCode::OK);
    let listed_body: serde_json::Value = read_json(list_response).await;
    assert_eq!(listed_body[0]["display_name"], "Desk Cluster");
    assert_eq!(listed_body[0]["ui_order"], 25);
    assert_eq!(listed_body[0]["controller"]["label"], "Desk wireless");

    let requests = mock.join();
    assert_eq!(requests.len(), 5);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
    assert!(matches!(requests[1], IpcRequest::ListDevices));
    assert!(matches!(requests[2], IpcRequest::GetTelemetry));
    assert!(matches!(requests[3], IpcRequest::ListDevices));
    assert!(matches!(requests[4], IpcRequest::GetTelemetry));
}
#[tokio::test]
async fn lighting_state_returns_existing_zone_config() {
    let device_id = "wireless:test:lighting";
    let path = format!("/api/devices/{}/lighting", encoded_device_id(device_id));
    let config = sample_device_config(device_id);
    let devices = vec![device_info(device_id, "RGB Device", true, true)];
    let mock = MockDaemon::new(2, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, &path))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["device_id"], device_id);
    assert_eq!(body["zones"][0]["zone"], 0);
    assert_eq!(body["zones"][0]["effect"], "Static");
    assert_eq!(body["zones"][0]["colors"], json!(["#112233"]));

    let requests = mock.join();
    assert_eq!(requests.len(), 2);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
    assert!(matches!(requests[1], IpcRequest::GetConfig));
}

#[tokio::test]
async fn lighting_color_endpoint_updates_rgb_config() {
    let device_id = "wireless:test:lighting";
    let path = format!(
        "/api/devices/{}/lighting/color",
        encoded_device_id(device_id)
    );
    let config = sample_device_config(device_id);
    let devices = vec![device_info(device_id, "RGB Device", true, true)];
    let mock = MockDaemon::new(3, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::SetRgbConfig { .. } => IpcResponse::ok(json!(null)),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(
            Method::POST,
            &path,
            json!({
                "zone": 0,
                "color": { "hex": "#abcdef" }
            }),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["zones"][0]["colors"], json!(["#abcdef"]));
    assert_eq!(body["zones"][0]["effect"], "Static");

    let requests = mock.join();
    assert_eq!(requests.len(), 3);
    let IpcRequest::SetRgbConfig { config } = &requests[2] else {
        panic!("expected SetRgbConfig request");
    };
    assert_eq!(config.devices[0].device_id, device_id);
    assert_eq!(config.devices[0].zones[0].effect.colors[0], [0xab, 0xcd, 0xef]);
    assert_eq!(config.devices[0].zones[0].effect.mode, RgbMode::Static);
}

#[tokio::test]
async fn lighting_color_endpoint_emits_lighting_changed_event() {
    let device_id = "wireless:test:lighting";
    let path = format!(
        "/api/devices/{}/lighting/color",
        encoded_device_id(device_id)
    );
    let config = sample_device_config(device_id);
    let devices = vec![device_info(device_id, "RGB Device", true, true)];
    let mock = MockDaemon::new(3, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::SetRgbConfig { .. } => IpcResponse::ok(json!(null)),
        other => panic!("unexpected request: {other:?}"),
    });
    let events = EventHub::new();
    let mut receiver = events.subscribe();

    let response = mock
        .app_with_events(events)
        .oneshot(json_request(
            Method::POST,
            &path,
            json!({
                "zone": 0,
                "color": { "hex": "#abcdef" }
            }),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let event: WebEvent = receiver.try_recv().expect("receive lighting event");
    assert_eq!(event.event_type, "lighting.changed");
    assert_eq!(event.source, "api");
    assert_eq!(event.device_id.as_deref(), Some(device_id));
    assert_eq!(event.data["reason"], "color_set");
    assert_eq!(event.data["zone"], 0);
    assert_eq!(event.data["color"], "#abcdef");

    let requests = mock.join();
    assert_eq!(requests.len(), 3);
}

#[tokio::test]
async fn lighting_effect_endpoint_updates_effect_fields() {
    let device_id = "wireless:test:lighting";
    let path = format!(
        "/api/devices/{}/lighting/effect",
        encoded_device_id(device_id)
    );
    let config = sample_device_config(device_id);
    let devices = vec![device_info(device_id, "RGB Device", true, true)];
    let mock = MockDaemon::new(3, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::SetRgbConfig { .. } => IpcResponse::ok(json!(null)),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(
            Method::POST,
            &path,
            json!({
                "zone": 0,
                "effect": "Rainbow",
                "speed": 4,
                "brightness": 25,
                "color": { "rgb": { "r": 1, "g": 2, "b": 3 } },
                "direction": "Up",
                "scope": "Inner"
            }),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["zones"][0]["effect"], "Rainbow");
    assert_eq!(body["zones"][0]["brightness_percent"], 25);
    assert_eq!(body["zones"][0]["speed"], 4);
    assert_eq!(body["zones"][0]["direction"], "Up");
    assert_eq!(body["zones"][0]["scope"], "Inner");
    assert_eq!(body["zones"][0]["colors"], json!(["#010203"]));

    let requests = mock.join();
    let IpcRequest::SetRgbConfig { config } = &requests[2] else {
        panic!("expected SetRgbConfig request");
    };
    let effect = &config.devices[0].zones[0].effect;
    assert_eq!(effect.mode, RgbMode::Rainbow);
    assert_eq!(effect.colors[0], [1, 2, 3]);
    assert_eq!(effect.speed, 4);
    assert_eq!(effect.brightness, 1);
    assert_eq!(effect.direction, RgbDirection::Up);
    assert_eq!(effect.scope, RgbScope::Inner);
}

#[tokio::test]
async fn lighting_brightness_endpoint_updates_brightness_only() {
    let device_id = "wireless:test:lighting";
    let path = format!(
        "/api/devices/{}/lighting/brightness",
        encoded_device_id(device_id)
    );
    let config = sample_device_config(device_id);
    let devices = vec![device_info(device_id, "RGB Device", true, true)];
    let mock = MockDaemon::new(3, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::SetRgbConfig { .. } => IpcResponse::ok(json!(null)),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(
            Method::POST,
            &path,
            json!({
                "zone": 0,
                "percent": 100
            }),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["zones"][0]["brightness_percent"], 100);

    let requests = mock.join();
    let IpcRequest::SetRgbConfig { config } = &requests[2] else {
        panic!("expected SetRgbConfig request");
    };
    assert_eq!(config.devices[0].zones[0].effect.brightness, 4);
}

#[tokio::test]
async fn lighting_brightness_endpoint_rejects_percent_over_100() {
    let device_id = "wireless:test:lighting";
    let path = format!(
        "/api/devices/{}/lighting/brightness",
        encoded_device_id(device_id)
    );
    let devices = vec![device_info(device_id, "RGB Device", true, true)];
    let mock = MockDaemon::new(1, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(
            Method::POST,
            &path,
            json!({
                "zone": 0,
                "percent": 101
            }),
        ))
        .await
        .expect("send request");

    assert_api_error(
        response,
        StatusCode::BAD_REQUEST,
        "BAD_REQUEST",
        "bad request: brightness percent must be 0-100",
    )
    .await;

    let requests = mock.join();
    assert_eq!(requests.len(), 1);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
}

#[tokio::test]
async fn lighting_zone_layout_get_preserves_led_order() {
    let device_id = "wireless:test:zones";
    let path = format!(
        "/api/devices/{}/lighting/zone-layout",
        encoded_device_id(device_id)
    );
    let mut config = sample_device_config(device_id);
    config.rgb = Some(RgbAppConfig {
        enabled: true,
        openrgb_server: false,
        openrgb_port: 6743,
        global_led_zones: vec![
            RgbLedZoneConfig {
                zone_index: 0,
                led_indexes: vec![7, 3, 5, 1],
            },
            RgbLedZoneConfig {
                zone_index: 1,
                led_indexes: vec![12, 9, 10],
            },
        ],
        fan_led_zones: Vec::new(),
        effect_route: Vec::new(),
        devices: Vec::new(),
    });
    let mut wireless_device = device_info(device_id, "Wireless Cluster", true, true);
    wireless_device.wireless_channel = Some(8);
    wireless_device.fan_count = Some(3);
    let capabilities = vec![rgb_capabilities(device_id, 3, 44)];
    let devices = vec![wireless_device];
    let mock = MockDaemon::new(3, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetRgbCapabilities => IpcResponse::ok(capabilities.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, &path))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["zones"][0]["led_indexes"], json!([7, 3, 5, 1]));
    assert_eq!(body["zones"][1]["led_indexes"], json!([12, 9, 10]));

    let requests = mock.join();
    assert_eq!(requests.len(), 3);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
    assert!(matches!(requests[1], IpcRequest::GetRgbCapabilities));
    assert!(matches!(requests[2], IpcRequest::GetConfig));
}

#[tokio::test]
async fn lighting_zone_layout_get_reads_requested_fan_slot_first() {
    let device_id = "wireless:test:zones";
    let path = format!(
        "/api/devices/{}/lighting/zone-layout?fan_index=2",
        encoded_device_id(device_id)
    );
    let mut config = sample_device_config(device_id);
    config.rgb = Some(RgbAppConfig {
        enabled: true,
        openrgb_server: false,
        openrgb_port: 6743,
        global_led_zones: vec![RgbLedZoneConfig {
            zone_index: 0,
            led_indexes: vec![0, 1, 2],
        }],
        fan_led_zones: vec![RgbFanLedZoneConfig {
            device_id: device_id.to_string(),
            fan_index: 2,
            zones: vec![
                RgbLedZoneConfig {
                    zone_index: 0,
                    led_indexes: vec![7, 3, 5, 1],
                },
                RgbLedZoneConfig {
                    zone_index: 1,
                    led_indexes: vec![12, 9, 10],
                },
            ],
        }],
        effect_route: Vec::new(),
        devices: Vec::new(),
    });
    let mut wireless_device = device_info(device_id, "Wireless Cluster", true, true);
    wireless_device.wireless_channel = Some(8);
    wireless_device.fan_count = Some(3);
    let capabilities = vec![rgb_capabilities(device_id, 3, 44)];
    let devices = vec![wireless_device];
    let mock = MockDaemon::new(3, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetRgbCapabilities => IpcResponse::ok(capabilities.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, &path))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["zones"][0]["led_indexes"], json!([7, 3, 5, 1]));
    assert_eq!(body["zones"][1]["led_indexes"], json!([12, 9, 10]));
}

#[tokio::test]
async fn lighting_zone_layout_get_falls_back_to_legacy_layout_when_fan_not_saved() {
    let device_id = "wireless:test:zones";
    let path = format!(
        "/api/devices/{}/lighting/zone-layout?fan_index=3",
        encoded_device_id(device_id)
    );
    let mut config = sample_device_config(device_id);
    config.rgb = Some(RgbAppConfig {
        enabled: true,
        openrgb_server: false,
        openrgb_port: 6743,
        global_led_zones: vec![
            RgbLedZoneConfig {
                zone_index: 0,
                led_indexes: vec![7, 3, 5, 1],
            },
            RgbLedZoneConfig {
                zone_index: 1,
                led_indexes: vec![12, 9, 10],
            },
        ],
        fan_led_zones: vec![RgbFanLedZoneConfig {
            device_id: device_id.to_string(),
            fan_index: 2,
            zones: vec![RgbLedZoneConfig {
                zone_index: 0,
                led_indexes: vec![40, 41],
            }],
        }],
        effect_route: Vec::new(),
        devices: Vec::new(),
    });
    let mut wireless_device = device_info(device_id, "Wireless Cluster", true, true);
    wireless_device.wireless_channel = Some(8);
    wireless_device.fan_count = Some(3);
    let capabilities = vec![rgb_capabilities(device_id, 3, 44)];
    let devices = vec![wireless_device];
    let mock = MockDaemon::new(3, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetRgbCapabilities => IpcResponse::ok(capabilities.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, &path))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["zones"][0]["led_indexes"], json!([7, 3, 5, 1]));
    assert_eq!(body["zones"][1]["led_indexes"], json!([12, 9, 10]));
}

#[tokio::test]
async fn lighting_zone_layout_save_stably_deduplicates_led_order() {
    let device_id = "wireless:test:zones";
    let path = format!(
        "/api/devices/{}/lighting/zone-layout",
        encoded_device_id(device_id)
    );
    let config = sample_device_config(device_id);
    let mut wireless_device = device_info(device_id, "Wireless Cluster", true, true);
    wireless_device.wireless_channel = Some(8);
    wireless_device.fan_count = Some(3);
    let capabilities = vec![rgb_capabilities(device_id, 3, 44)];
    let devices = vec![wireless_device];
    let mock = MockDaemon::new(4, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetRgbCapabilities => IpcResponse::ok(capabilities.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::SetConfig { .. } => IpcResponse::ok(json!(null)),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(
            Method::POST,
            &path,
            json!({
                "fan_index": 2,
                "zones": [
                    { "zone": 0, "led_indexes": [7, 3, 5, 7, 1] },
                    { "zone": 1, "led_indexes": [5, 12, 9, 12, 10] },
                    { "zone": 2, "led_indexes": [] },
                    { "zone": 3, "led_indexes": [] },
                    { "zone": 4, "led_indexes": [] }
                ]
            }),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["zones"][0]["led_indexes"], json!([7, 3, 5, 1]));
    assert_eq!(body["zones"][1]["led_indexes"], json!([12, 9, 10]));

    let requests = mock.join();
    let IpcRequest::SetConfig { config } = &requests[3] else {
        panic!("expected SetConfig request");
    };
    let rgb = config.rgb.as_ref().expect("rgb config");
    assert_eq!(rgb.fan_led_zones.len(), 1);
    assert_eq!(rgb.fan_led_zones[0].device_id, device_id);
    assert_eq!(rgb.fan_led_zones[0].fan_index, 2);
    assert_eq!(rgb.fan_led_zones[0].zones[0].led_indexes, vec![7, 3, 5, 1]);
    assert_eq!(rgb.fan_led_zones[0].zones[1].led_indexes, vec![12, 9, 10]);
}

#[tokio::test]
async fn lighting_workbench_apply_persists_rgb_config() {
    let device_id = "wireless:test:lighting:workbench";
    let config = sample_device_config(device_id);
    let devices = vec![device_info(device_id, "RGB Workbench Device", true, true)];
    let mock = MockDaemon::new(3, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::SetRgbConfig { .. } => IpcResponse::ok(json!(null)),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(
            Method::POST,
            "/api/lighting/apply",
            json!({
                "target_mode": "selected",
                "device_id": device_id,
                "device_ids": [device_id],
                "zone_mode": "all_zones",
                "effect": "Static",
                "brightness": 60,
                "speed": 2,
                "colors": [{ "hex": "#abcdef" }],
                "direction": "Clockwise",
                "scope": "All",
                "sync_selected": false
            }),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["applied_devices"][0]["device_id"], device_id);
    assert_eq!(body["applied_devices"][0]["zones"][0]["colors"], json!(["#abcdef"]));

    let requests = mock.join();
    let IpcRequest::SetRgbConfig { config } = &requests[2] else {
        panic!("expected SetRgbConfig request");
    };
    assert_eq!(config.devices[0].device_id, device_id);
    assert_eq!(config.devices[0].zones[0].effect.colors[0], [0xab, 0xcd, 0xef]);
    assert_eq!(config.devices[0].zones[0].effect.mode, RgbMode::Static);
}

#[tokio::test]
async fn lighting_effect_route_roundtrips_in_saved_order() {
    let config = sample_device_config("wireless:test:route");
    let mock = MockDaemon::new(2, move |request| match request {
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::SetRgbConfig { config } => IpcResponse::ok(json!(config)),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(
            Method::PUT,
            "/api/lighting/effect-route",
            json!({
                "route": [
                    { "device_id": "wireless:cluster-b", "fan_index": 2 },
                    { "device_id": "wireless:cluster-a", "fan_index": 1 },
                    { "device_id": "wireless:cluster-b", "fan_index": 3 }
                ]
            }),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(
        body["route"],
        json!([
            { "device_id": "wireless:cluster-b", "fan_index": 2 },
            { "device_id": "wireless:cluster-a", "fan_index": 1 },
            { "device_id": "wireless:cluster-b", "fan_index": 3 }
        ])
    );

    let requests = mock.join();
    let IpcRequest::SetRgbConfig { config } = &requests[1] else {
        panic!("expected SetRgbConfig request");
    };
    assert_eq!(config.effect_route.len(), 3);
    assert_eq!(config.effect_route[0].device_id, "wireless:cluster-b");
    assert_eq!(config.effect_route[0].fan_index, 2);
    assert_eq!(config.effect_route[1].device_id, "wireless:cluster-a");
    assert_eq!(config.effect_route[1].fan_index, 1);
    assert_eq!(config.effect_route[2].device_id, "wireless:cluster-b");
    assert_eq!(config.effect_route[2].fan_index, 3);
}

#[tokio::test]
async fn lighting_effect_route_rejects_duplicate_entries() {
    let config = sample_device_config("wireless:test:route");
    let mock = MockDaemon::new(1, move |request| match request {
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(
            Method::PUT,
            "/api/lighting/effect-route",
            json!({
                "route": [
                    { "device_id": "wireless:cluster-a", "fan_index": 1 },
                    { "device_id": "wireless:cluster-a", "fan_index": 1 }
                ]
            }),
        ))
        .await
        .expect("send request");

    assert_api_error(
        response,
        StatusCode::BAD_REQUEST,
        "BAD_REQUEST",
        "bad request: duplicate lighting effect route entry for wireless:cluster-a fan 1",
    )
    .await;
}

#[tokio::test]
async fn lighting_workbench_route_target_uses_saved_route_and_skips_missing_devices() {
    let device_id = "wireless:test:lighting:route";
    let mut config = sample_device_config(device_id);
    if let Some(rgb) = config.rgb.as_mut() {
        rgb.effect_route = vec![
            lianli_shared::rgb::RgbEffectRouteEntry {
                device_id: "wireless:missing".to_string(),
                fan_index: 1,
            },
            lianli_shared::rgb::RgbEffectRouteEntry {
                device_id: device_id.to_string(),
                fan_index: 2,
            },
        ];
    }
    let devices = vec![device_info(device_id, "RGB Route Device", true, true)];
    let mock = MockDaemon::new(3, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::SetRgbConfig { .. } => IpcResponse::ok(json!(null)),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(
            Method::POST,
            "/api/lighting/apply",
            json!({
                "target_mode": "route",
                "device_ids": [],
                "zone_mode": "all_zones",
                "effect": "Meteor",
                "brightness": 60,
                "speed": 2,
                "colors": [{ "hex": "#abcdef" }],
                "direction": "Clockwise",
                "scope": "All",
                "sync_selected": false
            }),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(
        body["requested_device_ids"],
        json!(["wireless:missing", device_id])
    );
    assert_eq!(body["applied_devices"][0]["device_id"], device_id);
    assert_eq!(body["skipped_devices"][0]["device_id"], "wireless:missing");
}

#[tokio::test]
async fn fan_state_endpoint_returns_configured_slots_and_telemetry() {
    let device_id = "wireless:test:fan";
    let path = format!("/api/devices/{}/fans", encoded_device_id(device_id));
    let config = sample_device_config(device_id);
    let devices = vec![device_info(device_id, "Fan Device", true, true)];
    let telemetry = sample_telemetry(device_id);
    let mock = MockDaemon::new(3, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::GetTelemetry => IpcResponse::ok(telemetry.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, &path))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["device_id"], device_id);
    assert_eq!(body["update_interval_ms"], 900);
    assert_eq!(body["rpms"], json!([910, 920, 930, 940]));
    assert_eq!(body["slots"][0]["mode"], "manual");
    assert_eq!(body["slots"][0]["percent"], 42);

    let requests = mock.join();
    assert_eq!(requests.len(), 3);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
    assert!(matches!(requests[1], IpcRequest::GetConfig));
    assert!(matches!(requests[2], IpcRequest::GetTelemetry));
}

#[tokio::test]
async fn fan_state_endpoint_limits_wireless_slots_to_discovered_fan_count() {
    let device_id = "wireless:test:fan";
    let path = format!("/api/devices/{}/fans", encoded_device_id(device_id));
    let config = sample_device_config(device_id);
    let mut wireless_device = device_info(device_id, "Fan Device", true, true);
    wireless_device.wireless_channel = Some(8);
    wireless_device.fan_count = Some(3);
    let devices = vec![wireless_device];
    let telemetry = sample_telemetry(device_id);
    let mock = MockDaemon::new(3, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::GetTelemetry => IpcResponse::ok(telemetry.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, &path))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["slots"].as_array().map(Vec::len), Some(3));
    assert_eq!(body["rpms"], json!([910, 920, 930]));

    let requests = mock.join();
    assert_eq!(requests.len(), 3);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
    assert!(matches!(requests[1], IpcRequest::GetConfig));
    assert!(matches!(requests[2], IpcRequest::GetTelemetry));
}

#[tokio::test]
async fn refresh_wireless_discovery_endpoint_triggers_daemon_scan() {
    let mock = MockDaemon::new(1, move |request| match request {
        IpcRequest::RefreshWirelessDiscovery => {
            IpcResponse::ok(json!({ "refreshed": true, "device_count": 3 }))
        }
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::POST, "/api/wireless/discovery/refresh"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body, json!({ "refreshed": true, "device_count": 3 }));

    let requests = mock.join();
    assert_eq!(requests.len(), 1);
    assert!(matches!(requests[0], IpcRequest::RefreshWirelessDiscovery));
}

#[tokio::test]
async fn connect_wireless_device_endpoint_binds_available_device() {
    let device_id = "wireless:test:cluster";
    let path = format!(
        "/api/devices/{}/wireless/connect",
        encoded_device_id(device_id)
    );
    let mut wireless_device = device_info(device_id, "Wireless Cluster", true, true);
    wireless_device.wireless_channel = Some(8);
    wireless_device.wireless_binding_state = Some(WirelessBindingState::Available);
    let devices = vec![wireless_device];
    let mock = MockDaemon::new(2, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::BindWirelessDevice { device_id } => {
            IpcResponse::ok(json!({ "device_id": device_id, "connected": true }))
        }
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::POST, &path))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body, json!({ "device_id": device_id, "connected": true }));

    let requests = mock.join();
    assert_eq!(requests.len(), 2);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
    assert!(matches!(
        &requests[1],
        IpcRequest::BindWirelessDevice { device_id: request_device_id }
            if request_device_id == device_id
    ));
}

#[tokio::test]
async fn connect_wireless_device_endpoint_rejects_foreign_devices() {
    let device_id = "wireless:test:foreign";
    let path = format!(
        "/api/devices/{}/wireless/connect",
        encoded_device_id(device_id)
    );
    let mut wireless_device = device_info(device_id, "Wireless Cluster", true, true);
    wireless_device.wireless_channel = Some(8);
    wireless_device.wireless_binding_state = Some(WirelessBindingState::Foreign);
    let devices = vec![wireless_device];
    let mock = MockDaemon::new(1, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::POST, &path))
        .await
        .expect("send request");

    assert_api_error(
        response,
        StatusCode::BAD_REQUEST,
        "BAD_REQUEST",
        "bad request: device is currently paired to another controller",
    )
    .await;

    let requests = mock.join();
    assert_eq!(requests.len(), 1);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
}
#[tokio::test]
async fn disconnect_wireless_device_endpoint_unbinds_wireless_device() {
    let device_id = "wireless:test:cluster";
    let path = format!(
        "/api/devices/{}/wireless/disconnect",
        encoded_device_id(device_id)
    );
    let mut wireless_device = device_info(device_id, "Wireless Cluster", true, true);
    wireless_device.wireless_channel = Some(8);
    let devices = vec![wireless_device];
    let mock = MockDaemon::new(2, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::UnbindWirelessDevice { device_id } => {
            IpcResponse::ok(json!({ "device_id": device_id, "disconnected": true }))
        }
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::POST, &path))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body, json!({ "device_id": device_id, "disconnected": true }));

    let requests = mock.join();
    assert_eq!(requests.len(), 2);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
    assert!(matches!(
        &requests[1],
        IpcRequest::UnbindWirelessDevice { device_id: request_device_id }
            if request_device_id == device_id
    ));
}

#[tokio::test]
async fn disconnect_wireless_device_endpoint_rejects_wired_devices() {
    let device_id = "usb:test:controller";
    let path = format!(
        "/api/devices/{}/wireless/disconnect",
        encoded_device_id(device_id)
    );
    let devices = vec![device_info(device_id, "USB Controller", true, true)];
    let mock = MockDaemon::new(1, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::POST, &path))
        .await
        .expect("send request");

    assert_api_error(
        response,
        StatusCode::BAD_REQUEST,
        "BAD_REQUEST",
        "bad request: device is not a wireless device",
    )
    .await;

    let requests = mock.join();
    assert_eq!(requests.len(), 1);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
}

#[tokio::test]
async fn fan_manual_endpoint_updates_fan_config() {
    let device_id = "wireless:test:fan";
    let path = format!(
        "/api/devices/{}/fans/manual",
        encoded_device_id(device_id)
    );
    let config = sample_device_config(device_id);
    let devices = vec![device_info(device_id, "Fan Device", true, true)];
    let mock = MockDaemon::new(4, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::SetFanConfig { .. } => IpcResponse::ok(json!(null)),
        IpcRequest::GetTelemetry => IpcResponse::ok(TelemetrySnapshot::default()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(
            Method::POST,
            &path,
            json!({
                "percent": 55,
                "slot": 2
            }),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["device_id"], device_id);
    assert_eq!(body["slots"][1]["mode"], "manual");
    assert_eq!(body["slots"][1]["percent"], 55);
    assert_eq!(body["rpms"], serde_json::Value::Null);

    let requests = mock.join();
    let IpcRequest::SetFanConfig { config } = &requests[2] else {
        panic!("expected SetFanConfig request");
    };
    let group = config
        .speeds
        .iter()
        .find(|group| group.device_id.as_deref() == Some(device_id))
        .expect("fan group");
    match &group.speeds[1] {
        FanSpeed::Constant(pwm) => assert_eq!(*pwm, 140),
        other => panic!("expected manual fan speed, got {other:?}"),
    }
}

#[tokio::test]
async fn fan_manual_endpoint_clamps_single_wireless_slinf_to_stable_minimum() {
    let device_id = "wireless:test:single";
    let path = format!(
        "/api/devices/{}/fans/manual",
        encoded_device_id(device_id)
    );
    let config = sample_device_config(device_id);
    let devices = vec![DeviceInfo {
        device_id: device_id.to_string(),
        family: DeviceFamily::SlInf,
        name: "Single SL-INF".to_string(),
        serial: None,
        wireless_channel: Some(8),
        wireless_missed_polls: None,
        wireless_master_mac: Some("3b:59:87:e5:66:e4".to_string()),
        wireless_binding_state: Some(WirelessBindingState::Connected),
        has_lcd: false,
        has_fan: true,
        has_pump: false,
        has_rgb: true,
        fan_count: Some(1),
        per_fan_control: Some(false),
        mb_sync_support: false,
        rgb_zone_count: Some(3),
        screen_width: None,
        screen_height: None,
    }];
    let mock = MockDaemon::new(4, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::SetFanConfig { .. } => IpcResponse::ok(json!(null)),
        IpcRequest::GetTelemetry => IpcResponse::ok(TelemetrySnapshot::default()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(
            Method::POST,
            &path,
            json!({
                "percent": 20
            }),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["slots"][0]["percent"], 30);
    assert_eq!(body["slots"][0]["pwm"], 77);

    let requests = mock.join();
    let IpcRequest::SetFanConfig { config } = &requests[2] else {
        panic!("expected SetFanConfig request");
    };
    let group = config
        .speeds
        .iter()
        .find(|group| group.device_id.as_deref() == Some(device_id))
        .expect("fan group");
    match &group.speeds[0] {
        FanSpeed::Constant(pwm) => assert_eq!(*pwm, 77),
        other => panic!("expected manual fan speed, got {other:?}"),
    }
}

fn rgb_capabilities(device_id: &str, fan_count: u8, leds_per_fan: u16) -> RgbDeviceCapabilities {
    RgbDeviceCapabilities {
        device_id: device_id.to_string(),
        device_name: "UNI FAN SL-INF Wireless".to_string(),
        supported_modes: vec![RgbMode::Static, RgbMode::Direct],
        zones: Vec::new(),
        supports_direct: true,
        supports_mb_rgb_sync: false,
        total_led_count: fan_count as u16 * leds_per_fan,
        supported_scopes: Vec::new(),
        supports_direction: false,
    }
}

#[tokio::test]
async fn fan_workbench_apply_persists_fan_config() {
    let device_id = "wireless:test:fan:workbench";
    let config = sample_device_config(device_id);
    let devices = vec![device_info(device_id, "Fan Workbench Device", true, true)];
    let mock = MockDaemon::new(4, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(config.clone()),
        IpcRequest::SetFanConfig { .. } => IpcResponse::ok(json!(null)),
        IpcRequest::GetTelemetry => IpcResponse::ok(TelemetrySnapshot::default()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(
            Method::POST,
            "/api/fans/apply",
            json!({
                "target_mode": "selected",
                "device_id": device_id,
                "device_ids": [device_id],
                "mode": "manual",
                "percent": 45
            }),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["applied_devices"][0]["device_id"], device_id);
    assert_eq!(body["applied_devices"][0]["slots"][0]["percent"], 45);

    let requests = mock.join();
    let IpcRequest::SetFanConfig { config } = &requests[2] else {
        panic!("expected SetFanConfig request");
    };
    let group = config
        .speeds
        .iter()
        .find(|group| group.device_id.as_deref() == Some(device_id))
        .expect("fan group");
    assert!(group
        .speeds
        .iter()
        .all(|speed| matches!(speed, FanSpeed::Constant(115))));
}

#[tokio::test]
async fn fan_manual_endpoint_rejects_invalid_percent_and_slot() {
    let device_id = "wireless:test:fan";
    let path = format!(
        "/api/devices/{}/fans/manual",
        encoded_device_id(device_id)
    );

    for (payload, message) in [
        (
            json!({
                "percent": 101
            }),
            "bad request: fan percent must be 0-100",
        ),
        (
            json!({
                "percent": 55,
                "slot": 5
            }),
            "bad request: slot must be 1-4",
        ),
    ] {
        let devices = vec![device_info(device_id, "Fan Device", true, true)];
        let mock = MockDaemon::new(1, move |request| match request {
            IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
            other => panic!("unexpected request: {other:?}"),
        });

        let response = mock
            .app()
            .oneshot(json_request(Method::POST, &path, payload))
            .await
            .expect("send request");

        assert_api_error(
            response,
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            message,
        )
        .await;

        let requests = mock.join();
        assert_eq!(requests.len(), 1);
        assert!(matches!(requests[0], IpcRequest::ListDevices));
    }
}

#[tokio::test]
async fn daemon_status_reports_reachable_when_ping_succeeds() {
    let mock = MockDaemon::new(1, |request| match request {
        IpcRequest::Ping => IpcResponse::ok(serde_json::json!("pong")),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(
            Request::builder()
                .uri("/api/daemon/status")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["reachable"], true);
    assert_eq!(body["socket_path"], mock.socket_path.display().to_string());

    let requests = mock.join();
    assert_eq!(requests.len(), 1);
    assert!(matches!(requests[0], IpcRequest::Ping));
}

#[tokio::test]
async fn daemon_status_reports_unreachable_when_transport_fails() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let missing_socket_path = mock._tempdir.path().join("missing-daemon.sock");
    let app = mock.app_with_config(
        BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9000,
            socket_path: missing_socket_path.clone(),
            log_level: "info".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: mock._tempdir.path().join("config.json"),
            profile_store_path: mock._tempdir.path().join("profiles.json"),
            auth: AuthConfig {
                mode: AuthMode::None,
                basic_username: None,
                basic_password: None,
                bearer_token: None,
                proxy_header: None,
            },
        },
        EventHub::new(),
    );

    let response = app
        .oneshot(empty_request(Method::GET, "/api/daemon/status"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["reachable"], false);
    assert_eq!(body["socket_path"], missing_socket_path.display().to_string());
    assert!(
        body["error"]
            .as_str()
            .expect("daemon error")
            .starts_with("io error:")
    );

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn runtime_reports_profile_store_path() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));

    let response = mock
        .app()
        .oneshot(
            Request::builder()
                .uri("/api/runtime")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["backend"]["port"], json!(9000));
    assert_eq!(
        body["backend"]["config_path"],
        json!(mock._tempdir.path().join("backend.json").display().to_string())
    );
    assert_eq!(
        body["backend"]["profile_store_path"],
        json!(mock._tempdir.path().join("profiles.json").display().to_string())
    );

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn runtime_redacts_auth_secrets_but_reports_auth_configuration() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let profile_store_path = mock._tempdir.path().join("profiles.json");
    let state = AppState {
        daemon: DaemonClient::new(mock.socket_path.clone()),
        config: BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9443,
            socket_path: mock.socket_path.clone(),
            log_level: "warn".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: mock._tempdir.path().join("config.json"),
            profile_store_path: profile_store_path.clone(),
            auth: AuthConfig {
                mode: AuthMode::Basic,
                basic_username: Some("admin".to_string()),
                basic_password: Some("super-secret".to_string()),
                bearer_token: None,
                proxy_header: None,
            },
        },
        profiles: ProfileStore::new(profile_store_path),
        events: EventHub::new(),
    };

    let token = STANDARD.encode("admin:super-secret");
    let response = routes::router(state)
        .oneshot(request_with_header(
            Method::GET,
            "/api/runtime",
            AUTHORIZATION.as_str(),
            &format!("Basic {token}"),
        ))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = read_json(response).await;
    assert_eq!(body["backend"]["port"], 9443);
    assert_eq!(body["backend"]["log_level"], "warn");
    assert_eq!(body["backend"]["auth"]["enabled"], true);
    assert_eq!(body["backend"]["auth"]["mode"], "basic");
    assert_eq!(body["backend"]["auth"]["reload_requires_restart"], true);
    assert_eq!(body["backend"]["auth"]["basic_username_configured"], true);
    assert_eq!(body["backend"]["auth"]["basic_password_configured"], true);
    assert_eq!(body["backend"]["auth"]["token_configured"], false);
    assert!(body["backend"]["auth"]["basic_password"].is_null());
    assert!(body["backend"]["auth"]["bearer_token"].is_null());

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn websocket_endpoint_streams_published_events() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let events = EventHub::new();
    let app = mock.app_with_events(events.clone());
    let (addr, server) = spawn_test_server(app).await;

    let (mut socket, response) = connect_async(format!("ws://{addr}/api/ws"))
        .await
        .expect("connect websocket");
    assert_eq!(response.status(), 101);

    events.publish_lighting_changed(
        "test",
        "wireless:test",
        json!({
            "reason": "unit_test",
            "zone": 0,
        }),
    );

    let payload = receive_websocket_text(&mut socket).await;
    let event: WebEvent = serde_json::from_str(&payload).expect("parse websocket event");
    assert_eq!(event.event_type, "lighting.changed");
    assert_eq!(event.source, "test");
    assert_eq!(event.device_id.as_deref(), Some("wireless:test"));
    assert_eq!(event.data["reason"], "unit_test");
    assert_eq!(event.data["zone"], 0);

    socket.close(None).await.expect("close websocket");
    server.abort();
    let _ = server.await;

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn websocket_endpoint_ignores_client_text_and_still_streams_events() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let events = EventHub::new();
    let app = mock.app_with_events(events.clone());
    let (addr, server) = spawn_test_server(app).await;

    let (mut socket, response) = connect_async(format!("ws://{addr}/api/ws"))
        .await
        .expect("connect websocket");
    assert_eq!(response.status(), 101);

    socket
        .send(WsMessage::Text("{\"ignored\":true}".to_string()))
        .await
        .expect("send client text");

    events.publish_fan_changed(
        "test",
        "wireless:test",
        json!({
            "reason": "unit_test",
            "percent": 42,
        }),
    );

    let payload = receive_websocket_text(&mut socket).await;
    let event: WebEvent = serde_json::from_str(&payload).expect("parse websocket event");
    assert_eq!(event.event_type, "fan.changed");
    assert_eq!(event.source, "test");
    assert_eq!(event.device_id.as_deref(), Some("wireless:test"));
    assert_eq!(event.data["reason"], "unit_test");
    assert_eq!(event.data["percent"], 42);

    socket.close(None).await.expect("close websocket");
    server.abort();
    let _ = server.await;

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn config_get_maps_daemon_config_to_web_document() {
    let daemon_config = sample_daemon_config();
    let mock = MockDaemon::new(1, move |request| match request {
        IpcRequest::GetConfig => IpcResponse::ok(daemon_config.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(
            Request::builder()
                .uri("/api/config")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: ConfigDocument = read_json(response).await;
    assert_eq!(body.default_fps, 24.0);
    assert_eq!(body.hid_driver, "rusb");
    assert_eq!(body.lighting.devices[0].zones[0].colors[0], "#112233");
    assert_eq!(body.fans.devices[0].slots[0].percent, Some(42));
    assert_eq!(body.lcds[0].device_id.as_deref(), Some("serial:LCD123"));
    assert_eq!(body.lcds[0].orientation, 90.0);

    let requests = mock.join();
    assert_eq!(requests.len(), 1);
    assert!(matches!(requests[0], IpcRequest::GetConfig));
}

#[tokio::test]
async fn config_get_maps_missing_rgb_and_fans_to_default_documents() {
    let mut daemon_config = sample_daemon_config();
    daemon_config.rgb = None;
    daemon_config.fans = None;
    let mock = MockDaemon::new(1, move |request| match request {
        IpcRequest::GetConfig => IpcResponse::ok(daemon_config.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, "/api/config"))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: ConfigDocument = read_json(response).await;
    assert!(body.lighting.enabled);
    assert!(!body.lighting.openrgb_server);
    assert_eq!(body.lighting.openrgb_port, 6743);
    assert!(body.lighting.devices.is_empty());
    assert_eq!(body.fans.update_interval_ms, 1000);
    assert!(body.fans.curves.is_empty());
    assert!(body.fans.devices.is_empty());

    let requests = mock.join();
    assert_eq!(requests.len(), 1);
    assert!(matches!(requests[0], IpcRequest::GetConfig));
}

#[tokio::test]
async fn config_get_returns_bad_gateway_when_daemon_replies_with_error() {
    let mock = MockDaemon::new(1, |request| match request {
        IpcRequest::GetConfig => IpcResponse::Error {
            message: "config unavailable".to_string(),
        },
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(empty_request(Method::GET, "/api/config"))
        .await
        .expect("send request");

    assert_api_error(
        response,
        StatusCode::BAD_GATEWAY,
        "DAEMON_ERROR",
        "daemon error: config unavailable",
    )
    .await;

    let requests = mock.join();
    assert_eq!(requests.len(), 1);
    assert!(matches!(requests[0], IpcRequest::GetConfig));
}

#[tokio::test]
async fn config_post_translates_web_document_to_daemon_config() {
    let response_document = sample_config_document();
    let mock = MockDaemon::new(1, |request| match request {
        IpcRequest::SetConfig { .. } => IpcResponse::ok(serde_json::json!(null)),
        other => panic!("unexpected request: {other:?}"),
    });

    let response = mock
        .app()
        .oneshot(json_request(Method::POST, "/api/config", &response_document))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: ConfigDocument = read_json(response).await;
    assert_eq!(body.hid_driver, "rusb");
    assert_eq!(body.lighting.devices[0].zones[0].brightness_percent, 75);
    assert_eq!(body.fans.devices[0].slots[0].percent, Some(42));
    assert_eq!(body.lcds[0].orientation, 90.0);

    let requests = mock.join();
    assert_eq!(requests.len(), 1);
    let IpcRequest::SetConfig { config } = &requests[0] else {
        panic!("expected SetConfig request");
    };

    assert_eq!(config.hid_driver, HidDriver::Rusb);
    assert_eq!(config.default_fps, 30.0);
    assert_eq!(config.lcds[0].device_id(), "serial:LCD123");
    assert_eq!(config.lcds[0].orientation, 90.0);

    let rgb = config.rgb.as_ref().expect("rgb config");
    assert_eq!(rgb.devices[0].zones[0].effect.colors[0], [0xab, 0xcd, 0xef]);
    assert_eq!(rgb.devices[0].zones[0].effect.brightness, 3);

    let fans = config.fans.as_ref().expect("fan config");
    match &fans.speeds[0].speeds[0] {
        FanSpeed::Constant(pwm) => assert_eq!(*pwm, 107),
        other => panic!("expected manual fan speed, got {other:?}"),
    }
}

#[tokio::test]
async fn config_post_validates_relative_lcd_paths_against_daemon_config_directory() {
    let config_dir = tempfile::tempdir().expect("create config directory");
    let daemon_config_path = config_dir.path().join("config").join("daemon.json");
    std::fs::create_dir_all(
        daemon_config_path
            .parent()
            .expect("daemon config parent directory"),
    )
    .expect("create daemon config parent directory");

    let media_path = daemon_config_path
        .parent()
        .expect("daemon config parent")
        .join("media")
        .join("lcd.png");
    let font_path = daemon_config_path
        .parent()
        .expect("daemon config parent")
        .join("fonts")
        .join("sensor.ttf");
    std::fs::create_dir_all(media_path.parent().expect("media dir")).expect("create media dir");
    std::fs::create_dir_all(font_path.parent().expect("font dir")).expect("create font dir");
    std::fs::write(&media_path, b"png").expect("write media file");
    std::fs::write(&font_path, b"font").expect("write font file");

    let mut response_document = sample_config_document();
    response_document.lcds = vec![
        LcdConfigDocument {
            device_id: Some("serial:LCD123".to_string()),
            index: None,
            serial: Some("LCD123".to_string()),
            media: "image".to_string(),
            path: Some("media/lcd.png".to_string()),
            fps: Some(12.0),
            color: None,
            orientation: 90.0,
            sensor: None,
        },
        LcdConfigDocument {
            device_id: Some("serial:LCD999".to_string()),
            index: None,
            serial: Some("LCD999".to_string()),
            media: "sensor".to_string(),
            path: None,
            fps: Some(15.0),
            color: None,
            orientation: 180.0,
            sensor: Some(SensorConfigDocument {
                label: "Coolant".to_string(),
                unit: "C".to_string(),
                source: SensorSourceDocument::Command {
                    command: "printf 42".to_string(),
                },
                text_color: "#ffffff".to_string(),
                background_color: "#000000".to_string(),
                gauge_background_color: "#111111".to_string(),
                ranges: vec![
                    SensorRangeDocument {
                        max: Some(50.0),
                        color: "#00ff00".to_string(),
                    },
                    SensorRangeDocument {
                        max: None,
                        color: "#ff0000".to_string(),
                    },
                ],
                update_interval_ms: 1_000,
                gauge_start_angle: 90.0,
                gauge_sweep_angle: 330.0,
                gauge_outer_radius: 180.0,
                gauge_thickness: 40.0,
                bar_corner_radius: 0.0,
                value_font_size: 72.0,
                unit_font_size: 32.0,
                label_font_size: 28.0,
                font_path: Some("fonts/sensor.ttf".to_string()),
                decimal_places: 0,
                value_offset: 0,
                unit_offset: 60,
                label_offset: -60,
            }),
        },
    ];

    let mock = MockDaemon::new(1, |request| match request {
        IpcRequest::SetConfig { .. } => IpcResponse::ok(serde_json::json!(null)),
        other => panic!("unexpected request: {other:?}"),
    });
    let app = mock.app_with_config(
        BackendConfig {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 9000,
            socket_path: mock.socket_path.clone(),
            log_level: "info".to_string(),
            backend_config_path: mock._tempdir.path().join("backend.json"),
            daemon_config_path: daemon_config_path.clone(),
            profile_store_path: mock._tempdir.path().join("profiles.json"),
            auth: AuthConfig {
                mode: AuthMode::None,
                basic_username: None,
                basic_password: None,
                bearer_token: None,
                proxy_header: None,
            },
        },
        EventHub::new(),
    );

    let response = app
        .oneshot(json_request(Method::POST, "/api/config", &response_document))
        .await
        .expect("send request");

    assert_eq!(response.status(), StatusCode::OK);

    let requests = mock.join();
    assert_eq!(requests.len(), 1);
    let IpcRequest::SetConfig { config } = &requests[0] else {
        panic!("expected SetConfig request");
    };

    assert_eq!(
        config.lcds[0]
            .path
            .as_ref()
            .expect("resolved media path")
            .as_path()
            .display()
            .to_string(),
        "media/lcd.png"
    );
    assert_eq!(
        config.lcds[1]
            .sensor
            .as_ref()
            .and_then(|sensor| sensor.font_path.as_ref())
            .expect("resolved font path")
            .as_path()
            .display()
            .to_string(),
        "fonts/sensor.ttf"
    );
}

#[tokio::test]
async fn config_post_rejects_invalid_fields_and_duplicate_lcd_targets() {
    let cases = vec![
        (
            {
                let mut invalid = sample_config_document();
                invalid.hid_driver = "usbfs".to_string();
                invalid
            },
            "bad request: unknown hid_driver: usbfs",
        ),
        (
            {
                let mut invalid = sample_config_document();
                invalid.lighting.devices[0].zones[0].effect = "Hyperdrive".to_string();
                invalid
            },
            "bad request: unknown effect: Hyperdrive",
        ),
        (
            {
                let mut invalid = sample_config_document();
                invalid.lighting.devices[0].zones[0].direction = "Sideways".to_string();
                invalid
            },
            "bad request: unknown direction: Sideways",
        ),
        (
            {
                let mut invalid = sample_config_document();
                invalid.lighting.devices[0].zones[0].scope = "Middle".to_string();
                invalid
            },
            "bad request: unknown scope: Middle",
        ),
        (
            {
                let mut invalid = sample_config_document();
                invalid.lighting.devices[0].zones[0].colors[0] = "#12345".to_string();
                invalid
            },
            "bad request: hex color must be 6 digits (RRGGBB)",
        ),
        (
            {
                let mut invalid = sample_config_document();
                invalid.lcds.push(LcdConfigDocument {
                    device_id: Some("serial:LCD123".to_string()),
                    index: None,
                    serial: Some("LCD123".to_string()),
                    media: "color".to_string(),
                    path: None,
                    fps: Some(12.0),
                    color: Some("#abcdef".to_string()),
                    orientation: 0.0,
                    sensor: None,
                });
                invalid
            },
            "bad request: duplicate LCD target 'serial:LCD123'",
        ),
    ];

    for (invalid, message) in cases {
        let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
        let response = mock
            .app()
            .oneshot(json_request(Method::POST, "/api/config", &invalid))
            .await
            .expect("send request");

        assert_api_error(response, StatusCode::BAD_REQUEST, "BAD_REQUEST", message).await;

        let requests = mock.join();
        assert!(requests.is_empty());
    }
}

#[tokio::test]
async fn config_post_rejects_duplicate_fan_slots() {
    let mut invalid = sample_config_document();
    invalid.fans.devices[0].slots[3].slot = 1;
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));

    let response = mock
        .app()
        .oneshot(json_request(Method::POST, "/api/config", &invalid))
        .await
        .expect("send request");

    assert_api_error(
        response,
        StatusCode::BAD_REQUEST,
        "BAD_REQUEST",
        "bad request: duplicate fan slot 1",
    )
    .await;

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn profile_crud_roundtrip_persists_to_json_store() {
    let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
    let create_request = sample_profile_request();

    let create_response = mock
        .app()
        .oneshot(json_request(Method::POST, "/api/profiles", &create_request))
        .await
        .expect("send create request");

    assert_eq!(create_response.status(), StatusCode::OK);
    let created: ProfileDocument = read_json(create_response).await;
    assert_eq!(created.id, "night-mode");
    assert_eq!(created.name, "Night Mode");
    assert_eq!(created.metadata.created_at, created.metadata.updated_at);

    let list_response = mock
        .app()
        .oneshot(
            Request::builder()
                .uri("/api/profiles")
                .body(Body::empty())
                .expect("build list request"),
        )
        .await
        .expect("send list request");

    assert_eq!(list_response.status(), StatusCode::OK);
    let profiles: Vec<ProfileDocument> = read_json(list_response).await;
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].id, "night-mode");

    let mut update_request = sample_profile_request();
    update_request.name = "Night Shift".to_string();
    update_request.fans.as_mut().unwrap().percent = Some(30);
    let update_response = mock
        .app()
        .oneshot(json_request(
            Method::PUT,
            "/api/profiles/night-mode",
            &update_request,
        ))
        .await
        .expect("send update request");

    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: ProfileDocument = read_json(update_response).await;
    assert_eq!(updated.name, "Night Shift");
    assert_eq!(updated.fans.as_ref().and_then(|fans| fans.percent), Some(30));
    assert_eq!(updated.metadata.created_at, created.metadata.created_at);
    assert_ne!(updated.metadata.updated_at, "");

    let delete_response = mock
        .app()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/profiles/night-mode")
                .body(Body::empty())
                .expect("build delete request"),
        )
        .await
        .expect("send delete request");

    assert_eq!(delete_response.status(), StatusCode::OK);
    let deleted: serde_json::Value = read_json(delete_response).await;
    assert_eq!(deleted["deleted"], true);
    assert_eq!(deleted["id"], "night-mode");

    let final_list_response = mock
        .app()
        .oneshot(
            Request::builder()
                .uri("/api/profiles")
                .body(Body::empty())
                .expect("build final list request"),
        )
        .await
        .expect("send final list request");

    assert_eq!(final_list_response.status(), StatusCode::OK);
    let final_profiles: Vec<ProfileDocument> = read_json(final_list_response).await;
    assert!(final_profiles.is_empty());

    let requests = mock.join();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn profile_endpoints_reject_invalid_payloads() {
    let cases = vec![
        (
            Method::POST,
            "/api/profiles".to_string(),
            json!({
                "id": "Night Mode",
                "name": "Night Mode",
                "targets": {
                    "mode": "all",
                    "device_ids": []
                },
                "lighting": {
                    "enabled": true,
                    "color": "#112233"
                }
            }),
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "bad request: profile id must contain only lowercase letters, numbers, '-' or '_'",
        ),
        (
            Method::POST,
            "/api/profiles".to_string(),
            json!({
                "id": "night-mode",
                "name": "Night Mode",
                "targets": {
                    "mode": "devices",
                    "device_ids": []
                },
                "lighting": {
                    "enabled": true,
                    "color": "#112233"
                }
            }),
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "bad request: targets.device_ids must not be empty when mode is 'devices'",
        ),
        (
            Method::POST,
            "/api/profiles".to_string(),
            json!({
                "id": "night-mode",
                "name": "Night Mode",
                "targets": {
                    "mode": "all",
                    "device_ids": []
                }
            }),
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "bad request: profile must include lighting and/or fans",
        ),
        (
            Method::POST,
            "/api/profiles".to_string(),
            json!({
                "id": "night-mode",
                "name": "Night Mode",
                "targets": {
                    "mode": "all",
                    "device_ids": []
                },
                "fans": {
                    "enabled": true,
                    "mode": "manual",
                    "percent": 101
                }
            }),
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "bad request: profile fan percent must be 0-100",
        ),
        (
            Method::PUT,
            "/api/profiles/night-mode".to_string(),
            json!({
                "id": "other-mode",
                "name": "Night Mode",
                "targets": {
                    "mode": "all",
                    "device_ids": []
                },
                "lighting": {
                    "enabled": true,
                    "color": "#112233"
                }
            }),
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "bad request: profile id in path and body must match",
        ),
    ];

    for (method, uri, payload, status, code, message) in cases {
        let mock = MockDaemon::new(0, |_| panic!("daemon should not be called"));
        let response = mock
            .app()
            .oneshot(json_request(method, &uri, payload))
            .await
            .expect("send request");

        assert_api_error(response, status, code, message).await;

        let requests = mock.join();
        assert!(requests.is_empty());
    }
}

#[tokio::test]
async fn profile_apply_batches_changes_into_single_config_write() {
    let stored_profile = sample_profile_request();
    let daemon_config = sample_daemon_config();
    let devices = vec![
        device_info("wireless:rgb", "RGB Device", true, true),
        device_info("wireless:fan", "Fan Device", false, true),
    ];

    let mock = MockDaemon::new(3, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        IpcRequest::GetConfig => IpcResponse::ok(daemon_config.clone()),
        IpcRequest::SetConfig { .. } => IpcResponse::ok(serde_json::json!(null)),
        other => panic!("unexpected request: {other:?}"),
    });

    let create_response = mock
        .app()
        .oneshot(json_request(Method::POST, "/api/profiles", &stored_profile))
        .await
        .expect("store profile");
    assert_eq!(create_response.status(), StatusCode::OK);

    let apply_response = mock
        .app()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/profiles/night-mode/apply")
                .body(Body::empty())
                .expect("build apply request"),
        )
        .await
        .expect("apply profile");

    assert_eq!(apply_response.status(), StatusCode::OK);
    let body: ProfileApplyResponse = read_json(apply_response).await;
    assert_eq!(body.profile_id, "night-mode");
    assert_eq!(body.transaction_mode, "single_config_write");
    assert_eq!(body.applied_lighting_device_ids, vec!["wireless:rgb"]);
    assert_eq!(
        body.applied_fan_device_ids,
        vec!["wireless:rgb", "wireless:fan"]
    );
    assert_eq!(body.skipped_devices.len(), 1);
    assert_eq!(body.skipped_devices[0].device_id, "wireless:fan");
    assert_eq!(body.skipped_devices[0].section, "lighting");

    let requests = mock.join();
    assert_eq!(requests.len(), 3);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
    assert!(matches!(requests[1], IpcRequest::GetConfig));

    let IpcRequest::SetConfig { config } = &requests[2] else {
        panic!("expected SetConfig request");
    };

    let rgb = config.rgb.as_ref().expect("rgb config");
    let rgb_device = rgb
        .devices
        .iter()
        .find(|device| device.device_id == "wireless:rgb")
        .expect("rgb device config");
    assert_eq!(rgb_device.zones[0].effect.mode, RgbMode::Static);
    assert_eq!(rgb_device.zones[0].effect.colors[0], [0x22, 0x33, 0x66]);
    assert_eq!(rgb_device.zones[0].effect.brightness, 1);

    let fans = config.fans.as_ref().expect("fan config");
    assert!(fans
        .speeds
        .iter()
        .any(|group| group.device_id.as_deref() == Some("wireless:rgb")));
    let fan_only_group = fans
        .speeds
        .iter()
        .find(|group| group.device_id.as_deref() == Some("wireless:fan"))
        .expect("fan-only device config");
    match &fan_only_group.speeds[0] {
        FanSpeed::Constant(pwm) => assert_eq!(*pwm, 64),
        other => panic!("expected manual fan speed, got {other:?}"),
    }
}

#[tokio::test]
async fn profile_apply_returns_not_found_for_unknown_target_device() {
    let mut stored_profile = sample_profile_request();
    stored_profile.targets.mode = "devices".to_string();
    stored_profile.targets.device_ids = vec!["wireless:missing".to_string()];
    let devices = vec![device_info("wireless:rgb", "RGB Device", true, true)];
    let mock = MockDaemon::new(1, move |request| match request {
        IpcRequest::ListDevices => IpcResponse::ok(devices.clone()),
        other => panic!("unexpected request: {other:?}"),
    });

    let create_response = mock
        .app()
        .oneshot(json_request(Method::POST, "/api/profiles", &stored_profile))
        .await
        .expect("store profile");
    assert_eq!(create_response.status(), StatusCode::OK);

    let apply_response = mock
        .app()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/profiles/night-mode/apply")
                .body(Body::empty())
                .expect("build apply request"),
        )
        .await
        .expect("apply profile");

    assert_api_error(
        apply_response,
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
        "not found: profile target device not found: wireless:missing",
    )
    .await;

    let requests = mock.join();
    assert_eq!(requests.len(), 1);
    assert!(matches!(requests[0], IpcRequest::ListDevices));
}













