//! STA 模式：UDP 广播（hello/heartbeat）+ TCP 控制服务（设计 5.2）+ 局域网配对

use log::{info, warn};
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddrV4, TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::thread;
use std::time::Duration;

use crate::hardware::{ContinuousServoService, ServoService};
use crate::system::config::{
    STA_TCP_PORT, STA_UDP_PORT, SERVO2_CMD_NEUTRAL, TOF_PERIOD_FAST_MS, TOF_PERIOD_SLOW_MS,
    TOF_THRESHOLD_FAST_MM, TOF_THRESHOLD_START_MM,
};

/// 绑定状态：是否已绑定、绑定的手机 ID
pub type BindingState = Arc<Mutex<(bool, Option<String>)>>;

/// 配对状态：待处理列表、已配对设备 ID、对方 IP（用于 remote_trigger）
pub type PairState = Arc<Mutex<(Vec<(String, String)>, Option<String>, Option<String>)>>;

/// 被 pair 设备远程触发的防抖与计数：(last_trigger_time_ms, triggered_count)
pub type TriggerState = Arc<Mutex<(u64, u32)>>;

/// B 端是否正在接收 A 的同步触发（用于 servo1/servo2 同步线程的启动/停止）
pub type SyncRunning = Arc<Mutex<bool>>;

/// Flutter 同步的 WiFi 列表：(ssid, password, auth)。每次与 Flutter 通讯后可更新，供后续重连等使用。
pub type WifiListStore = Arc<Mutex<Vec<(String, Option<String>, String)>>>;

fn lock_or_recover<'a, T>(mutex: &'a Mutex<T>, name: &str) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(g) => g,
        Err(e) => {
            warn!("mutex '{}' poisoned, continue with inner value", name);
            e.into_inner()
        }
    }
}

fn error_response(code: &str, message: &str) -> String {
    serde_json::json!({
        "status": "error",
        "code": code,
        "message": message,
        // backward compatibility for old clients that only parse `reason`
        "reason": code,
    })
    .to_string()
}

/// 收到配网成功（STA 已连接）时，启动 UDP 广播与 TCP 控制（各占一线程）
pub fn spawn_sta_services_on_connect(
    device_id: String,
    sta_ip: String,
    bind_token: Option<String>,
    sta_ssid: Option<String>,
    servo: Option<Arc<ServoService>>,
    servo2: Option<Arc<ContinuousServoService>>,
    binding_state: BindingState,
    pair_state: PairState,
    trigger_state: TriggerState,
    sync_running: SyncRunning,
    wifi_list_store: WifiListStore,
) {
    let did = device_id.clone();
    let ip = sta_ip.clone();
    let binding_state_udp = Arc::clone(&binding_state);
    thread::Builder::new()
        .name("sta-udp".into())
        .spawn(move || run_udp_broadcast(did, ip, bind_token, sta_ssid, binding_state_udp))
        .expect("spawn sta-udp");
        thread::Builder::new()
            .name("sta-tcp".into())
            .spawn(move || run_tcp_control(device_id, sta_ip, servo, servo2, binding_state, pair_state, trigger_state, sync_running, wifi_list_store))
            .expect("spawn sta-tcp");
}

const BINDING_INTERVAL_SECS: u64 = 4;
const MAX_PENDING_PAIR_REQUESTS: usize = 64;

fn run_udp_broadcast(
    device_id: String,
    sta_ip: String,
    bind_token: Option<String>,
    sta_ssid: Option<String>,
    binding_state: BindingState,
) {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => {
            warn!("STA UDP bind failed: {}", e);
            return;
        }
    };
    let _ = socket.set_broadcast(true);
    let dest: SocketAddrV4 = SocketAddrV4::new(std::net::Ipv4Addr::new(255, 255, 255, 255), STA_UDP_PORT);
    loop {
        let (bound, _) = *lock_or_recover(binding_state.as_ref(), "binding_state");
        if bound {
            let msg = serde_json::json!({
                "evt": "heartbeat",
                "id": device_id,
                "ssid": sta_ssid.as_deref().unwrap_or(""),
            });
            let _ = socket.send_to(msg.to_string().as_bytes(), dest);
        } else if let Some(ref token) = bind_token {
            let msg = serde_json::json!({
                "evt": "binding",
                "id": device_id,
                "ip": sta_ip,
                "ssid": sta_ssid.as_deref().unwrap_or(""),
                "bindToken": token,
            });
            let _ = socket.send_to(msg.to_string().as_bytes(), dest);
        } else {
            // Keep discoverability even when no bind token is available.
            let msg = serde_json::json!({
                "evt": "hello",
                "id": device_id,
                "ip": sta_ip,
                "ssid": sta_ssid.as_deref().unwrap_or(""),
            });
            let _ = socket.send_to(msg.to_string().as_bytes(), dest);
        }
        thread::sleep(Duration::from_secs(BINDING_INTERVAL_SECS));
    }
}

fn run_tcp_control(
    device_id: String,
    _sta_ip: String,
    servo: Option<Arc<ServoService>>,
    servo2: Option<Arc<ContinuousServoService>>,
    binding_state: BindingState,
    pair_state: PairState,
    trigger_state: TriggerState,
    sync_running: SyncRunning,
    wifi_list_store: WifiListStore,
) {
    let addr = format!("0.0.0.0:{}", STA_TCP_PORT);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            warn!("STA TCP {} bind failed: {}", addr, e);
            return;
        }
    };
    info!("STA TCP control on {}", addr);
    let active_clients = Arc::new(AtomicUsize::new(0));
    const MAX_ACTIVE_CLIENTS: usize = 6;
    for stream in listener.incoming().filter_map(Result::ok) {
        let peer_addr = stream.peer_addr().ok();
        if let Some(ref peer) = peer_addr {
            info!("STA TCP client connected from {}", peer);
        }
        let cur = active_clients.load(AtomicOrdering::Relaxed);
        if cur >= MAX_ACTIVE_CLIENTS {
            // Too many connections; drop this one to avoid OOM/thread explosion.
            warn!("STA TCP too many active clients ({}), dropping connection", cur);
            let mut s = stream;
            let _ = writeln!(
                s,
                "{}",
                error_response("busy", "too_many_connections")
            );
            let _ = s.flush();
            let _ = s.shutdown(Shutdown::Both);
            continue;
        }

        let servo = servo.clone();
        let servo2 = servo2.clone();
        let binding_state = Arc::clone(&binding_state);
        let pair_state = Arc::clone(&pair_state);
        let wifi_list_store = Arc::clone(&wifi_list_store);
        let trigger_state = Arc::clone(&trigger_state);
        let sync_running = Arc::clone(&sync_running);
        let device_id = device_id.clone();
        let active_clients = Arc::clone(&active_clients);
        let _ = thread::Builder::new()
            .name("sta-client".into())
            .stack_size(64 * 1024)
            .spawn(move || {
            active_clients.fetch_add(1, AtomicOrdering::Relaxed);
            let _ = handle_sta_client(
                stream,
                &device_id,
                &servo,
                &servo2,
                &binding_state,
                &pair_state,
                &trigger_state,
                &sync_running,
                &wifi_list_store,
                peer_addr,
            );
            active_clients.fetch_sub(1, AtomicOrdering::Relaxed);
        });
    }
}

/// 从 stream 读一行（到 \n 或 EOF），不依赖 try_clone（ESP32 上常不可用）
fn read_line(stream: &mut std::net::TcpStream) -> std::io::Result<String> {
    let mut line = Vec::new();
    let mut buf = [0u8; 1];
    loop {
        let n = match stream.read(&mut buf) {
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::TimedOut => continue,
            Err(e) => return Err(e),
        };
        if n == 0 {
            break;
        }
        if buf[0] == b'\n' {
            break;
        }
        if buf[0] != b'\r' {
            line.push(buf[0]);
        }
    }
    Ok(String::from_utf8_lossy(&line).into_owned())
}

fn handle_sta_client(
    mut stream: TcpStream,
    device_id: &str,
    servo: &Option<Arc<ServoService>>,
    servo2: &Option<Arc<ContinuousServoService>>,
    binding_state: &BindingState,
    pair_state: &PairState,
    trigger_state: &TriggerState,
    sync_running: &SyncRunning,
    wifi_list_store: &WifiListStore,
    peer_addr: Option<std::net::SocketAddr>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    loop {
        let line = match read_line(&mut stream) {
            Ok(s) => s,
            Err(e) => {
                // 对方断开或 ESP 资源类错误(如 11) 时少刷 warn；128 = not connected
                let ok_disconnect = matches!(
                    e.kind(),
                    ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::BrokenPipe
                        | ErrorKind::UnexpectedEof
                ) || e.raw_os_error() == Some(128);
                let resource_err = e.raw_os_error() == Some(11); // EAGAIN / 资源暂不可用
                if ok_disconnect {
                    info!("STA TCP client disconnected");
                } else if resource_err {
                    info!("STA TCP read: client left or busy ({})", e);
                } else {
                    warn!("STA TCP read error: {}", e);
                }
                return;
            }
        };

        let msg = line.trim();
        if msg.is_empty() {
            continue;
        }

        info!(
            "STA TCP cmd: {}",
            if msg.len() > 80 {
                format!("{}...", &msg[..80])
            } else {
                msg.to_string()
            }
        );
        let response = process_sta_command(
            msg,
            device_id,
            servo,
            servo2,
            binding_state,
            pair_state,
            trigger_state,
            sync_running,
            wifi_list_store,
            peer_addr,
        );
        if writeln!(stream, "{}", response).is_err() || stream.flush().is_err() {
            warn!("STA TCP write error");
            return;
        }
    }
}

fn process_sta_command(
    msg: &str,
    device_id: &str,
    servo: &Option<Arc<ServoService>>,
    servo2: &Option<Arc<ContinuousServoService>>,
    binding_state: &BindingState,
    pair_state: &PairState,
    trigger_state: &TriggerState,
    sync_running: &SyncRunning,
    wifi_list_store: &WifiListStore,
    peer_addr: Option<std::net::SocketAddr>,
) -> String {
    let json: serde_json::Value = match serde_json::from_str(msg) {
        Ok(j) => j,
        Err(_) => return error_response("invalid_json", "invalid json"),
    };
    let cmd = json.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
    match cmd {
        "demo_servo" => {
            if let Some(s) = servo {
                match s.demo_sequence() {
                    Ok(()) => r#"{"status":"ok"}"#.to_string(),
                    Err(e) => error_response("servo_error", &e.to_string()),
                }
            } else {
                error_response("servo_unavailable", "servo unavailable")
            }
        }
        "move_servo" => {
            let idx = json.get("servo").and_then(|n| n.as_u64()).unwrap_or(0);
            let angle = json.get("angle").and_then(|n| n.as_u64()).unwrap_or(90) as u16;
            if idx == 0 {
                match servo.as_ref() {
                    Some(s) => match s.set_angle(angle.min(300)) {
                        Ok(()) => r#"{"status":"ok"}"#.to_string(),
                        Err(e) => error_response("servo_error", &e.to_string()),
                    },
                    None => error_response("servo_unavailable", "servo unavailable"),
                }
            } else {
                match servo2.as_ref() {
                    Some(s) => match s.set_angle(angle.min(180)) {
                        Ok(()) => r#"{"status":"ok"}"#.to_string(),
                        Err(e) => error_response("servo_error", &e.to_string()),
                    },
                    None => error_response("servo_unavailable", "servo unavailable"),
                }
            }
        }
        "bind" => {
            let phone = json
                .get("phone")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();
            let mut state = lock_or_recover(binding_state.as_ref(), "binding_state");
            *state = (true, Some(phone));
            r#"{"status":"ok"}"#.to_string()
        }
        "unbind" => {
            let mut state = lock_or_recover(binding_state.as_ref(), "binding_state");
            *state = (false, None);
            r#"{"status":"ok"}"#.to_string()
        }
        "pair_request" => {
            if let (Some(target_ip), Some(target_device_id)) = (
                json.get("target_ip").and_then(|v| v.as_str()),
                json.get("target_device_id").and_then(|v| v.as_str()),
            ) {
                let addr = format!("{}:{}", target_ip, STA_TCP_PORT);
                let parsed_addr = match addr.parse() {
                    Ok(a) => a,
                    Err(_) => {
                        return error_response("invalid_target_addr", "invalid target address");
                    }
                };
                if let Ok(mut other) = TcpStream::connect_timeout(&parsed_addr, Duration::from_secs(5)) {
                    let req = serde_json::json!({"cmd":"pair_request","from_device_id": device_id});
                    let _ = writeln!(other, "{}", req);
                    let _ = other.flush();
                    let mut line = Vec::new();
                    let mut buf = [0u8; 1];
                    let _ = other.set_read_timeout(Some(Duration::from_secs(3)));
                    while other.read(&mut buf).ok() == Some(1) {
                        if buf[0] == b'\n' {
                            break;
                        }
                        if buf[0] != b'\r' {
                            line.push(buf[0]);
                        }
                    }
                    if line.is_empty() {
                        return error_response(
                            "empty_target_response",
                            "empty response from target device",
                        );
                    }
                    let resp_str = String::from_utf8_lossy(&line).to_string();
                    let resp_json: serde_json::Value = match serde_json::from_str(&resp_str) {
                        Ok(v) => v,
                        Err(_) => {
                            return error_response(
                                "invalid_target_response",
                                "invalid json from target device",
                            )
                        }
                    };
                    let status = resp_json.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    let message = resp_json.get("message").and_then(|v| v.as_str()).unwrap_or("");
                    if status == "ok" && message == "pair_accepted" {
                        let mut state = lock_or_recover(pair_state.as_ref(), "pair_state");
                        state.1 = Some(target_device_id.to_string());
                        let peer_ip = if let Some(v) = resp_json.get("peer_ip").and_then(|v| v.as_str()) {
                            if !v.is_empty() { v.to_string() } else { target_ip.to_string() }
                        } else {
                            target_ip.to_string()
                        };
                        state.2 = Some(peer_ip.clone());
                        return serde_json::json!({
                            "status":"ok",
                            "message":"pair_accepted",
                            "peer_ip": peer_ip
                        }).to_string();
                    }
                    if status == "ok" && message == "pending" {
                        // Pending is not paired yet, but we still know the peer IP.
                        let mut state = lock_or_recover(pair_state.as_ref(), "pair_state");
                        state.1 = None;
                        state.2 = Some(target_ip.to_string());
                        return serde_json::json!({"status":"ok","message":"pending"}).to_string();
                    }
                    if status == "error" {
                        let code = resp_json.get("code").and_then(|v| v.as_str()).unwrap_or("target_error");
                        let reason = resp_json
                            .get("reason")
                            .or_else(|| resp_json.get("message"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("target returned error");
                        return error_response(code, reason);
                    }
                    return error_response(
                        "unexpected_target_response",
                        "unexpected target response status/message",
                    );
                }
                return error_response(
                    "connect_target_failed",
                    "timeout or refused when connecting target",
                );
            }
            if let Some(from_id) = json.get("from_device_id").and_then(|v| v.as_str()) {
                let from_ip = peer_addr
                    .and_then(|a| a.to_string().split(':').next().map(String::from))
                    .unwrap_or_else(|| "0.0.0.0".to_string());
                let mut state = lock_or_recover(pair_state.as_ref(), "pair_state");
                state.0.retain(|(id, _)| id != from_id);
                if state.0.len() >= MAX_PENDING_PAIR_REQUESTS {
                    let drop_n = state.0.len() - MAX_PENDING_PAIR_REQUESTS + 1;
                    state.0.drain(0..drop_n);
                }
                state.0.push((from_id.to_string(), from_ip));
                return r#"{"status":"ok","message":"pending"}"#.to_string();
            }
            error_response(
                "missing_target_ip_or_from_device_id",
                "missing target_ip or from_device_id",
            )
        }
        "get_pending_pair_requests" => {
            let state = lock_or_recover(pair_state.as_ref(), "pair_state");
            let pending: Vec<serde_json::Value> = state
                .0
                .iter()
                .map(|(id, ip)| serde_json::json!({"from_device_id": id, "from_ip": ip}))
                .collect();
            serde_json::json!({"status":"ok","pending": pending}).to_string()
        }
        "get_pair_status" => {
            // Avoid holding two mutexes at once: copy pair state first, then read trigger state.
            let (paired_with_owned, peer_ip_owned) = {
                let pair_guard = lock_or_recover(pair_state.as_ref(), "pair_state");
                (
                    pair_guard.1.clone().unwrap_or_default(),
                    pair_guard.2.clone().unwrap_or_default(),
                )
            };
            let triggered_count = {
                let trig = lock_or_recover(trigger_state.as_ref(), "trigger_state");
                trig.1
            };
            serde_json::json!({
                "status":"ok",
                "paired_with": paired_with_owned,
                "peer_ip": peer_ip_owned,
                "triggered_count": triggered_count
            })
            .to_string()
        }
        "sync_start" => {
            // Start sync thread:
            // - Servo1 (limited-angle) moves with fixed "middle" speed.
            // - Servo2 (continuous rotation) keeps its built-in 10s periodic behavior.
            //
            // Trigger message only: no phase/velocity info is sent from A.

            // Change state first, and only spawn a new thread when transitioning false -> true.
            let spawn_thread = {
                let mut run = lock_or_recover(sync_running.as_ref(), "sync_running");
                if *run {
                    false
                } else {
                    *run = true;
                    true
                }
            };

            if !spawn_thread {
                return r#"{"status":"ok"}"#.to_string();
            }

            if let Some(s1) = servo2.as_ref() {
                let _ = s1.set_angle(SERVO2_CMD_NEUTRAL);
            }

            let s0 = servo.clone();
            let s1 = servo2.clone();
            let sync_running_for_thread = Arc::clone(sync_running);
            let trigger_state_for_thread = Arc::clone(trigger_state);

            let _ = std::thread::Builder::new()
                .name("sync-runner".into())
                .stack_size(96 * 1024)
                .spawn(move || {
                    let sync_logic = || {
                        const TICK_MS: u64 = 40;
                        const ANGLE_MIN: u16 = 25;
                        const ANGLE_MAX: u16 = 130;
                        const SPAN: u16 = 105;
                        const PHASE_FULL_CYCLE: f32 = 2.0;

                        const SERVO2_ROTATE_CMD: u16 = 120;
                        const SERVO2_ROTATE_DURATION_MS: u64 = 400;
                        const SERVO2_ROTATE_INTERVAL_MS: u64 = 10_000;

                        let mm_range_linear =
                            TOF_THRESHOLD_START_MM.saturating_sub(TOF_THRESHOLD_FAST_MM).max(1);
                        let d_mid =
                            ((TOF_THRESHOLD_START_MM as u32 + TOF_THRESHOLD_FAST_MM as u32) / 2) as u16;
                        let period_ms_mid = if d_mid <= TOF_THRESHOLD_FAST_MM {
                            TOF_PERIOD_FAST_MS
                        } else {
                            TOF_PERIOD_SLOW_MS
                                - (TOF_THRESHOLD_START_MM - d_mid) as u64
                                    * (TOF_PERIOD_SLOW_MS - TOF_PERIOD_FAST_MS)
                                    / (mm_range_linear as u64)
                        };
                        let period_ms_mid = period_ms_mid.max(1);

                        let mut phase: f32 = 0.0;
                        // Servo2 rotation window managed in-thread to avoid spawning short-lived threads.
                        let mut servo2_rotating_until_ms: Option<u64> = None;

                        loop {
                            let run_now =
                                *lock_or_recover(sync_running_for_thread.as_ref(), "sync_running");
                            if !run_now {
                                break;
                            }

                            std::thread::sleep(std::time::Duration::from_millis(TICK_MS));

                            // Check again after sleep for faster stop.
                            let run_now =
                                *lock_or_recover(sync_running_for_thread.as_ref(), "sync_running");
                            if !run_now {
                                break;
                            }

                            let advance = (TICK_MS as f32 / period_ms_mid as f32) * PHASE_FULL_CYCLE;
                            phase += advance;
                            if phase >= PHASE_FULL_CYCLE {
                                phase -= PHASE_FULL_CYCLE;
                            }

                            let angle0 = if phase < 1.0 {
                                ANGLE_MAX - (SPAN as f32 * phase) as u16
                            } else {
                                ANGLE_MIN + (SPAN as f32 * (phase - 1.0)) as u16
                            };

                            if let Some(ref servo0) = s0 {
                                let _ = servo0.set_angle(angle0);
                            }

                            // servo2 scheduler
                            if let Some(ref servo2_service) = s1 {
                                let now_ms = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64;

                                if let Some(until_ms) = servo2_rotating_until_ms {
                                    if now_ms >= until_ms {
                                        let _ = servo2_service.set_angle(SERVO2_CMD_NEUTRAL);
                                        servo2_rotating_until_ms = None;
                                    }
                                }

                                let should_rotate = {
                                    let mut trig = lock_or_recover(
                                        trigger_state_for_thread.as_ref(),
                                        "trigger_state",
                                    );
                                    let should =
                                        servo2_rotating_until_ms.is_none()
                                            && now_ms.saturating_sub(trig.0)
                                                >= SERVO2_ROTATE_INTERVAL_MS;
                                    if should {
                                        trig.0 = now_ms;
                                        trig.1 = trig.1.saturating_add(1);
                                    }
                                    should
                                };

                                if should_rotate {
                                    let _ = servo2_service.set_angle(SERVO2_ROTATE_CMD);
                                    servo2_rotating_until_ms =
                                        Some(now_ms.saturating_add(SERVO2_ROTATE_DURATION_MS));
                                }
                            }
                        }
                    };

                    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(sync_logic)).is_err();
                    if panicked {
                        warn!("sync-runner thread panicked, reset sync_running");
                    }
                    {
                        let mut run = lock_or_recover(sync_running_for_thread.as_ref(), "sync_running");
                        *run = false;
                    }
                    if let Some(ref servo2_service) = s1 {
                        let _ = servo2_service.set_angle(SERVO2_CMD_NEUTRAL);
                    }
                });

            r#"{"status":"ok"}"#.to_string()
        }
        "sync_stop" => {
            // Stop sync:
            // - stop sync thread
            // - park servo1 near idle angle
            // - stop servo2 rotation
            let mut run = lock_or_recover(sync_running.as_ref(), "sync_running");
            *run = false;

            if let Some(s0) = servo.as_ref() {
                let current = s0.get_angle();
                let d25 = (current as i32 - 25).abs() as u16;
                let d130 = (current as i32 - 130).abs() as u16;
                let idle_angle = if d25 <= d130 { 25 } else { 130 };
                let _ = s0.set_angle(idle_angle);
            }

            if let Some(s2) = servo2.as_ref() {
                let _ = s2.set_angle(SERVO2_CMD_NEUTRAL);
            }
            r#"{"status":"ok"}"#.to_string()
        }
        "accept_pair" => {
            let from_device_id = json.get("from_device_id").and_then(|v| v.as_str()).unwrap_or("");
            let mut state = lock_or_recover(pair_state.as_ref(), "pair_state");
            let from_ip = state.0.iter().find(|(id, _)| id == from_device_id).map(|(_, ip)| ip.clone());
            if let Some(ip) = from_ip.clone() {
                state.0.retain(|(id, _)| id != from_device_id);
                state.1 = Some(from_device_id.to_string());
                state.2 = Some(ip.clone());
                drop(state);
                let addr = format!("{}:{}", ip, STA_TCP_PORT);
                if let Ok(addr_parsed) = addr.parse() {
                    if let Ok(mut other) = TcpStream::connect_timeout(&addr_parsed, Duration::from_secs(5)) {
                    let req = serde_json::json!({
                        "cmd":"pair_accepted",
                        "device_id": device_id,
                        "peer_ip": ip
                    });
                    let _ = writeln!(other, "{}", req);
                    let _ = other.flush();
                }
                }
                return r#"{"status":"ok"}"#.to_string();
            }
            error_response("pending_not_found", "pending request not found")
        }
        "reject_pair" => {
            // Remove a pending pair request without changing paired state.
            let from_device_id = json.get("from_device_id").and_then(|v| v.as_str()).unwrap_or("");
            let mut state = lock_or_recover(pair_state.as_ref(), "pair_state");
            state.0.retain(|(id, _)| id != from_device_id);
            r#"{"status":"ok"}"#.to_string()
        }
        "pair_accepted" => {
            if let Some(peer_id) = json.get("device_id").and_then(|v| v.as_str()) {
                let mut state = lock_or_recover(pair_state.as_ref(), "pair_state");
                state.1 = Some(peer_id.to_string());
                // Ensure initiator stores the *real* peer_ip, not a potentially stale target_ip.
                // For pair_accepted, TCP client is the remote (acceptor) device.
                if let Some(req_ip) = json.get("peer_ip").and_then(|v| v.as_str()) {
                    if !req_ip.is_empty() {
                        state.2 = Some(req_ip.to_string());
                    }
                } else if let Some(pa) = peer_addr {
                    let ip = pa.to_string().split(':').next().unwrap_or("").to_string();
                    if !ip.is_empty() {
                        state.2 = Some(ip);
                    }
                }
                return r#"{"status":"ok"}"#.to_string();
            }
            error_response("missing_device_id", "missing device_id")
        }
        "unpair" => {
            let peer_ip_from_req = json.get("peer_ip").and_then(|v| v.as_str()).map(|s| s.to_string());
            let mut state = lock_or_recover(pair_state.as_ref(), "pair_state");
            // Prefer peer_ip provided by request; otherwise fall back to peer_ip stored in pair_state.
            let peer_ip = peer_ip_from_req.or_else(|| state.2.clone());

            // Clear local pair state first (we already captured peer_ip if available).
            state.1 = None;
            state.2 = None;
            state.0.clear();
            drop(state);

            if let Some(ip) = peer_ip {
                let addr = format!("{}:{}", ip, STA_TCP_PORT);
                if let Ok(addr_parsed) = addr.parse() {
                    if let Ok(mut other) = TcpStream::connect_timeout(&addr_parsed, Duration::from_secs(5)) {
                    let req = serde_json::json!({"cmd":"unpair_notify","device_id": device_id});
                    let _ = writeln!(other, "{}", req);
                    let _ = other.flush();
                }
                }
            }
            r#"{"status":"ok"}"#.to_string()
        }
        "unpair_notify" => {
            if json.get("device_id").is_some() {
                let mut state = lock_or_recover(pair_state.as_ref(), "pair_state");
                state.1 = None;
                state.2 = None;
                return r#"{"status":"ok"}"#.to_string();
            }
            error_response("missing_device_id", "missing device_id")
        }
        "update_wifi_list" => {
            if let Some(networks) = json.get("networks").and_then(|n| n.as_array()) {
                let list: Vec<(String, Option<String>, String)> = networks
                    .iter()
                    .filter_map(|n| {
                        let obj = n.as_object()?;
                        let ssid = obj.get("ssid").and_then(|s| s.as_str())?.to_string();
                        let pwd = obj.get("pwd").and_then(|s| s.as_str()).map(String::from);
                        let sec = obj.get("sec").and_then(|s| s.as_u64()).unwrap_or(3) as u8;
                        let auth = match sec {
                            0 => "open",
                            1 => "wep",
                            2 => "wpa",
                            3 => "wpa2",
                            4 => "wpa3",
                            5 => "wpa2_enterprise",
                            _ => "wpa2",
                        };
                        Some((ssid, pwd, auth.to_string()))
                    })
                    .collect();
                let mut store = lock_or_recover(wifi_list_store.as_ref(), "wifi_list_store");
                *store = list;
                return r#"{"status":"ok"}"#.to_string();
            }
            error_response("missing_networks", "missing networks")
        }
        _ => error_response("unknown_cmd", "unknown cmd"),
    }
}
