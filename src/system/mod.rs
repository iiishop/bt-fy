//! Butterfly System - Main orchestration module
//!
//! This module provides the main application logic.
//! It coordinates all services (WiFi, DNS, Web, Hardware) and manages their lifecycle.

#![allow(deprecated)] // DNS 模块已标记过时但仍保留类型，由 ENABLE_DNS_CAPTIVE 控制是否启动

pub mod config;

use esp_idf_hal::{
    ledc::{self, config::TimerConfig, LedcTimerDriver},
    peripheral::Peripheral,
    peripherals::Peripherals,
    prelude::*,
};
use esp_idf_hal::ledc::LowSpeed;
use esp_idf_svc::{eventloop::EspSystemEventLoop, log::EspLogger, nvs::{EspDefaultNvs, EspDefaultNvsPartition}};
use log::info;
use std::sync::Arc;

use crate::{
    dns::DnsService,
    hardware::{self, ContinuousServoService, vl53l1x::VL53L1XService, ServoService, TofSensor, VL53L0XService},
    protocol::{
        spawn_sta_services_on_connect, start_ap_tcp_listener, BindingState, PairState, PendingBindToken,
        PendingConfigDone, SyncRunning, TriggerState, WifiListStore, BindTokenStore,
    },
    system::config::{
        AP_IP_ADDRESS, ENABLE_DNS_CAPTIVE, SERVO2_PIN, SERVO_PIN,
        TOF_PERIOD_FAST_MS, TOF_PERIOD_SLOW_MS,
        TOF_THRESHOLD_FAST_MM, TOF_THRESHOLD_START_MM,
    },
    web::{HardwareStatus, WebService},
    wifi::{WifiCommand, WifiResponse, WifiService},
};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

/// 每 boot 只启动一次 STA 服务，避免重复 bind 12345 导致 Address already in use
static STA_SERVICES_STARTED: AtomicBool = AtomicBool::new(false);

/// Main system that orchestrates all services
pub struct ButterflySystem {
    _nvs: EspDefaultNvsPartition,
    wifi: WifiService,
    wifi_cmd_tx: Option<mpsc::Sender<(WifiCommand, mpsc::Sender<WifiResponse>)>>,
    wifi_cmd_rx: Option<mpsc::Receiver<(WifiCommand, mpsc::Sender<WifiResponse>)>>,
    dns: DnsService,
    web: WebService,
    sensor: Option<Arc<dyn TofSensor>>,
    servo: Option<Arc<ServoService>>,
    servo2: Option<Arc<ContinuousServoService>>,
}

impl ButterflySystem {
    /// Create a new Butterfly system with automatic fallback
    pub fn new(mut peripherals: Peripherals) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Initializing Butterfly System...");

        // Initialize ESP32 system
        Self::init_system()?;

        // Initialize NVS (Non-Volatile Storage) - required for WiFi
        let nvs = EspDefaultNvsPartition::take()?;
        info!("NVS initialized");

        // Get system event loop
        let sys_loop = EspSystemEventLoop::take()?;

        // Try to initialize hardware FIRST (before moving modem to WiFi)
        let (sensor, servo, servo2) = Self::try_init_hardware(&mut peripherals);

        // Now create core services (WiFi, DNS, Web)
        let wifi = WifiService::new(peripherals.modem, sys_loop)?;
        info!("WiFi service created");

        let (wifi_cmd_tx, wifi_cmd_rx) = mpsc::channel();
        let mut web = WebService::new(AP_IP_ADDRESS)?;
        web.set_wifi_cmd_tx(Some(wifi_cmd_tx.clone()));
        info!("Web service created");

        let dns = DnsService::new(AP_IP_ADDRESS)?;
        info!("DNS service created");

        if sensor.is_some() || servo.is_some() || servo2.is_some() {
            info!("✓ Hardware initialized successfully");
        } else {
            log::warn!("⚠ System running in CAPTIVE PORTAL ONLY mode");
            log::info!("→ WiFi and web interface still available");
        }

        Ok(Self {
            _nvs: nvs,
            wifi,
            wifi_cmd_tx: Some(wifi_cmd_tx),
            wifi_cmd_rx: Some(wifi_cmd_rx),
            dns,
            web,
            sensor,
            servo,
            servo2,
        })
    }

    /// Try to initialize hardware (VL53L0X + two servos on GPIO3 and GPIO4)
    /// Order: TOF first (I2C + background thread), then LEDC timer and servos, so I2C is stable.
    fn try_init_hardware(
        peripherals: &mut Peripherals,
    ) -> (Option<Arc<dyn TofSensor>>, Option<Arc<ServoService>>, Option<Arc<ContinuousServoService>>) {
        // 1) Init VL53L0X first so I2C and its reading thread start before any LEDC/GPIO3/4 activity
        let sensor: Option<Arc<dyn TofSensor>> = {
            info!("Initializing VL53L0X sensor...");

            let i2c_driver = match hardware::vl53l0x::create_i2c_driver(
                unsafe { peripherals.i2c0.clone_unchecked() },
                unsafe { peripherals.pins.gpio5.clone_unchecked() }, // SDA
                unsafe { peripherals.pins.gpio2.clone_unchecked() }, // SCL
            ) {
                Ok(driver) => driver,
                Err(e) => {
                    log::warn!("VL53L0X I2C init failed: {}", e);
                    return (None, None, None);
                }
            };

            match VL53L0XService::new(i2c_driver) {
                Ok(service) => {
                    info!("VL53L0X sensor ready");
                    Some(Arc::new(service) as Arc<dyn TofSensor>)
                }
                Err(e) => {
                    log::warn!("VL53L0X sensor init failed: {}", e);
                    // 尝试 VL53L1X 作为 fallback
                    log::warn!("Trying VL53L1X sensor as fallback...");
                    let i2c_driver_l1 = match hardware::vl53l1x::create_i2c_driver(
                        unsafe { peripherals.i2c0.clone_unchecked() },
                        unsafe { peripherals.pins.gpio5.clone_unchecked() },
                        unsafe { peripherals.pins.gpio2.clone_unchecked() },
                    ) {
                        Ok(driver) => driver,
                        Err(e2) => {
                            log::warn!("VL53L1X I2C init failed: {}", e2);
                            return (None, None, None);
                        }
                    };
                    match VL53L1XService::new(i2c_driver_l1) {
                        Ok(service) => {
                            info!("VL53L1X sensor ready");
                            Some(Arc::new(service) as Arc<dyn TofSensor>)
                        }
                        Err(e2) => {
                            log::warn!("VL53L1X sensor init failed: {}", e2);
                            None
                        }
                    }
                }
            }
        };

        // 2) Then shared LEDC timer and servos (GPIO3/4)
        const SERVO_PWM_HZ: u32 = 50;
        let timer_cfg = TimerConfig::new()
            .frequency(SERVO_PWM_HZ.Hz())
            .resolution(ledc::config::Resolution::Bits12);
        let timer_driver = match LedcTimerDriver::new(
            unsafe { peripherals.ledc.timer0.clone_unchecked() },
            &timer_cfg,
        ) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("LEDC timer init failed: {}", e);
                return (sensor, None, None);
            }
        };
        let shared_timer = Box::leak(Box::new(timer_driver));

        let servo = Self::try_init_servo(peripherals, shared_timer);
        let servo2 = Self::try_init_servo2(peripherals, shared_timer);
        (sensor, servo, servo2)
    }

    fn try_init_servo<T>(
        peripherals: &mut Peripherals,
        timer: &'static LedcTimerDriver<'static, T>,
    ) -> Option<Arc<ServoService>>
    where
        T: ledc::LedcTimer<SpeedMode = LowSpeed> + 'static,
    {
        match ServoService::new_with_shared_timer(
            unsafe { peripherals.ledc.channel0.clone_unchecked() },
            timer,
            unsafe { peripherals.pins.gpio3.clone_unchecked() },
            SERVO_PIN,
        ) {
            Ok(service) => {
                info!("DS-S006L servo (GPIO{}) ready", SERVO_PIN);
                Some(Arc::new(service))
            }
            Err(e) => {
                log::warn!("DS-S006L servo (GPIO{}) init failed: {}", SERVO_PIN, e);
                None
            }
        }
    }

    fn try_init_servo2<T>(
        peripherals: &mut Peripherals,
        timer: &'static LedcTimerDriver<'static, T>,
    ) -> Option<Arc<ContinuousServoService>>
    where
        T: ledc::LedcTimer<SpeedMode = LowSpeed> + 'static,
    {
        match ContinuousServoService::new_with_shared_timer(
            unsafe { peripherals.ledc.channel1.clone_unchecked() },
            timer,
            unsafe { peripherals.pins.gpio4.clone_unchecked() },
            SERVO2_PIN,
        ) {
            Ok(service) => {
                info!("DS-S006L servo (GPIO{}) ready", SERVO2_PIN);
                Some(Arc::new(service))
            }
            Err(e) => {
                log::warn!("DS-S006L servo (GPIO{}) init failed: {}", SERVO2_PIN, e);
                None
            }
        }
    }

    /// Run the system
    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting Butterfly System...");

        // Start WiFi first - foundation for everything
        self.wifi.start()?;
        info!("WiFi started");

        // DNS 截获已关闭（ENABLE_DNS_CAPTIVE = false），不再启动
        if ENABLE_DNS_CAPTIVE {
            self.dns.start()?;
            info!("DNS server started");
        } else {
            info!("DNS captive portal disabled (ENABLE_DNS_CAPTIVE=false)");
        }

        // 提前创建，供 test-mode（pair_state）与 sta-starter 使用
        let binding_state: BindingState = Arc::new(Mutex::new((false, None)));
        let pair_state: PairState = Arc::new(Mutex::new((vec![], None, None)));
        let trigger_state: TriggerState = Arc::new(Mutex::new((0u64, 0u32)));
        let sync_running: SyncRunning = Arc::new(Mutex::new(false));
        let wifi_list_store: WifiListStore = Arc::new(Mutex::new(vec![]));
        // Persist/restore binding state across power cycles.
        let nvs_handle: Arc<Mutex<EspDefaultNvsPartition>> = Arc::new(Mutex::new(self._nvs.clone()));

        // Configure hardware status for web service
        if self.sensor.is_some() || self.servo.is_some() || self.servo2.is_some() {
            let sensor = self.sensor.clone();
            let servo = self.servo.clone();
            let servo2 = self.servo2.clone();

            let distance = Arc::new(move || sensor.as_ref().map(|s| s.get_distance()).unwrap_or(0));
            let servo_for_angle = self.servo.clone();
            let servo_angle = Arc::new(move || {
                servo_for_angle
                    .as_ref()
                    .map(|s| s.get_angle())
                    .unwrap_or(90)
            });
            let servo_set = Arc::new(move |angle: u16| {
                servo
                    .as_ref()
                    .ok_or_else(|| "servo unavailable".to_string())
                    .and_then(|s| s.set_angle(angle))
            });
            let servo2_for_angle = self.servo2.clone();
            let servo2_angle = Arc::new(move || {
                servo2_for_angle
                    .as_ref()
                    .map(|s| s.get_angle())
                    .unwrap_or(90)
            });
            let servo2_set = Arc::new(move |angle: u16| {
                servo2
                    .as_ref()
                    .ok_or_else(|| "servo2 unavailable".to_string())
                    .and_then(|s| s.set_angle(angle))
            });

            let hw_status = HardwareStatus {
                distance: distance.clone(),
                servo_angle,
                servo_set: servo_set.clone(),
                servo2_angle,
                servo2_set: servo2_set.clone(),
            };

            // 靠近 ToF 时自动触发：pair 前仅舵机0 动、舵机1 不动；pair 后本机同上，并通知对方设备 remote_trigger
            let test_mode = Arc::new(AtomicBool::new(true));
            let distance_thd = distance.clone();
            let servo_set_thd = servo_set.clone();
            let servo2_set_thd = servo2_set.clone();
            let test_mode_thd = Arc::clone(&test_mode);
            let sync_running_thd = Arc::clone(&sync_running);
            let pair_state_thd = Arc::clone(&pair_state);
            let sta_tcp_port = crate::system::config::STA_TCP_PORT;

            // After pairing:
            // - Trigger: drive peer sync via an AtomicBool (desired running), not a depth-1 channel.
            //   sync_channel + try_send 会静默丢 Stop，导致对端 sync 停不下来。
            // - sync-sender 轮询 desired，与已成功写入 TCP 的 last_sent 比较；写失败则下轮重试。
            // - Servo speed on the peer is handled internally (fixed "middle" speed).
            let remote_peer_sync_desired = Arc::new(AtomicBool::new(false));
            let remote_desired_for_sender = Arc::clone(&remote_peer_sync_desired);
            let pair_state_for_sender = Arc::clone(&pair_state_thd);
            std::thread::Builder::new()
                .name("sync-sender".into())
                .spawn(move || {
                    // Keep one TCP connection to the peer and reuse it.
                    let mut stream_opt: Option<std::net::TcpStream> = None;
                    let mut stream_peer_ip: Option<String> = None;
                    // 最近一次收到对端 ACK 后确认的「是否在跑 remote sync」。
                    let mut last_confirmed_running = false;
                    // 用于让 ACK 能对应到一次请求。
                    let mut req_seq: u64 = 0;

                    loop {
                        std::thread::sleep(Duration::from_millis(50));

                        let desired = remote_desired_for_sender.load(Ordering::Acquire);

                        let (paired_with_opt, peer_ip_opt) = pair_state_for_sender
                            .lock()
                            .ok()
                            .map(|g| (g.1.clone(), g.2.clone()))
                            .unwrap_or((None, None));
                        let (Some(_paired_with), Some(peer_ip)) = (paired_with_opt, peer_ip_opt) else {
                            stream_opt = None;
                            stream_peer_ip = None;
                            last_confirmed_running = false;
                            continue;
                        };

                        if desired == last_confirmed_running {
                            continue;
                        }

                        let need_reconnect = match stream_peer_ip.as_deref() {
                            Some(ip) if ip == peer_ip => false,
                            _ => true,
                        } || stream_opt.is_none();

                        if need_reconnect {
                            stream_opt = None;
                            stream_peer_ip = Some(peer_ip.clone());

                            let addr = format!("{}:{}", peer_ip, sta_tcp_port);
                            if let Ok(addr_parsed) = addr.parse::<std::net::SocketAddr>() {
                                if let Ok(mut s) =
                                    std::net::TcpStream::connect_timeout(&addr_parsed, Duration::from_secs(2))
                                {
                                    let _ = s.set_write_timeout(Some(Duration::from_secs(1)));
                                    let _ = s.set_read_timeout(Some(Duration::from_millis(400)));
                                    stream_opt = Some(s);
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        }

                        let Some(stream) = stream_opt.as_mut() else {
                            continue;
                        };

                        req_seq = req_seq.wrapping_add(1);
                        let expected_ack = if desired { "sync_start" } else { "sync_stop" };
                        let cmd_line = if desired {
                            format!(r#"{{"cmd":"sync_start","req_id":{}}}"#, req_seq)
                        } else {
                            format!(r#"{{"cmd":"sync_stop","req_id":{}}}"#, req_seq)
                        };

                        if writeln!(stream, "{}", cmd_line).is_err() || stream.flush().is_err() {
                            stream_opt = None;
                            continue;
                        }

                        // Read one-line response to keep TCP recv buffer healthy.
                        let start = std::time::Instant::now();
                        let mut buf = [0u8; 1];
                        let mut resp = Vec::<u8>::new();
                        while start.elapsed() < Duration::from_millis(600) {
                            match std::io::Read::read(stream, &mut buf) {
                                Ok(0) => break,
                                Ok(_) => {
                                    if buf[0] == b'\n' {
                                        break;
                                    }
                                    if buf[0] != b'\r' {
                                        resp.push(buf[0]);
                                    }
                                }
                                Err(_) => break,
                            }
                        }

                        let resp_str = String::from_utf8_lossy(&resp).trim().to_string();
                        let mut acked = false;
                        if !resp_str.is_empty() {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&resp_str) {
                                let status_ok = v
                                    .get("status")
                                    .and_then(|x| x.as_str())
                                    .map(|s| s == "ok")
                                    .unwrap_or(false);
                                let ack_ok = v
                                    .get("ack")
                                    .and_then(|x| x.as_str())
                                    .map(|s| s == expected_ack)
                                    .unwrap_or(false);
                                let ack_req_id = v.get("req_id").and_then(|x| x.as_u64());
                                let req_match = ack_req_id == Some(req_seq);
                                acked = status_ok && ack_ok && req_match;
                            }
                        }

                        if acked {
                            last_confirmed_running = desired;
                        } else {
                            // 未收到匹配 ACK：认为链路当前不可靠，下轮继续重发（同 desired）。
                            stream_opt = None;
                        }
                    }
                })?;

            let remote_desired_for_test = Arc::clone(&remote_peer_sync_desired);
            std::thread::Builder::new()
                .name("test-mode".into())
                .spawn(move || {
                    // --- Test mode constants (from config + local) ---
                    const TICK_MS: u64 = 40;
                    const THRESHOLD_MIN_MM: u16 = 20;     // ignore 0/invalid (sensor disconnected)
                    let mm_range_linear = TOF_THRESHOLD_START_MM.saturating_sub(TOF_THRESHOLD_FAST_MM).max(1);
                    const SERVO1_ANGLE_IDLE: u16 = 90;  // pair 前/后本机 servo1 均保持 90° 不动
                    const SERVO2_ANGLE_MIN: u16 = 25;
                    const SERVO2_ANGLE_MAX: u16 = 130;
                    const SERVO2_ANGLE_SPAN: u16 = 105;   // SERVO2_ANGLE_MAX - SERVO2_ANGLE_MIN (130-25)
                    const PHASE_FULL_CYCLE: f32 = 2.0;   // phase 0..1: 130→25, 1..2: 25→130

                    // Continuous phase [0, 2): no reset when period changes, so speed changes smoothly
                    let mut phase: f32 = 0.0;
                    let mut was_triggered = false;
                    let mut idle_applied = false;

                    loop {
                        std::thread::sleep(Duration::from_millis(TICK_MS));
                        // When remote sync is running (B side), let sync_start thread own the servos.
                        // Avoid sensor-based test-mode from fighting with remote control.
                        if *sync_running_thd.lock().unwrap_or_else(|e| e.into_inner()) {
                            was_triggered = false;
                            idle_applied = false;
                            // 如果本机正在被远端 sync 控制，则不要继续把“我们曾经触发过的期望状态”
                            // 维持给对端；否则对端可能一直收到 sync_start/无法停止。
                            remote_desired_for_test.store(false, Ordering::Release);
                            continue;
                        }
                        if !test_mode_thd.load(Ordering::Relaxed) {
                            was_triggered = false;
                            idle_applied = false;
                            continue;
                        }
                        let d = (distance_thd)();
                        // Only trigger when sensor reports valid near range (ignore 0 = disconnected)
                        if d >= THRESHOLD_MIN_MM && d < TOF_THRESHOLD_START_MM {
                            if !was_triggered {
                                // rising edge: notify peer once
                                idle_applied = false;
                                was_triggered = true;
                                let paired_with_opt = pair_state_thd
                                    .lock()
                                    .ok()
                                    .and_then(|g| g.1.clone());
                                if paired_with_opt.is_some() {
                                    remote_desired_for_test.store(true, Ordering::Release);
                                }
                            } else {
                                idle_applied = false;
                            }
                            let period_ms = if d <= TOF_THRESHOLD_FAST_MM {
                                TOF_PERIOD_FAST_MS
                            } else {
                                TOF_PERIOD_SLOW_MS
                                    - (TOF_THRESHOLD_START_MM - d) as u64
                                        * (TOF_PERIOD_SLOW_MS - TOF_PERIOD_FAST_MS)
                                        / (mm_range_linear as u64)
                            };
                            let advance =
                                (TICK_MS as f32 / period_ms as f32) * PHASE_FULL_CYCLE;
                            phase += advance;
                            if phase >= PHASE_FULL_CYCLE {
                                phase -= PHASE_FULL_CYCLE;
                            }
                            let angle0 = if phase < 1.0 {
                                SERVO2_ANGLE_MAX
                                    - (SERVO2_ANGLE_SPAN as f32 * phase) as u16
                            } else {
                                SERVO2_ANGLE_MIN
                                    + (SERVO2_ANGLE_SPAN as f32 * (phase - 1.0)) as u16
                            };
                            let _ = (servo_set_thd)(angle0);
                            let _ = (servo2_set_thd)(SERVO1_ANGLE_IDLE);
                        } else {
                            if was_triggered {
                                // 根据当前 phase 算当前角度，离 25 或 130 哪个更近就停在哪
                                let current = if phase < 1.0 {
                                    SERVO2_ANGLE_MAX - (SERVO2_ANGLE_SPAN as f32 * phase) as u16
                                } else {
                                    SERVO2_ANGLE_MIN + (SERVO2_ANGLE_SPAN as f32 * (phase - 1.0)) as u16
                                };
                                let mid = (SERVO2_ANGLE_MIN + SERVO2_ANGLE_MAX) / 2;
                                let idle_angle = if current < mid { SERVO2_ANGLE_MIN } else { SERVO2_ANGLE_MAX };
                                let _ = (servo_set_thd)(idle_angle);
                                let _ = (servo2_set_thd)(SERVO1_ANGLE_IDLE); // 舵机1 回 90°
                                idle_applied = true;
                                phase = 0.0;
                                // falling edge: notify peer once
                                let paired_with_opt = pair_state_thd
                                    .lock()
                                    .ok()
                                    .and_then(|g| g.1.clone());
                                if paired_with_opt.is_some() {
                                    remote_desired_for_test.store(false, Ordering::Release);
                                }
                            }
                            was_triggered = false;
                            if !idle_applied {
                                let _ = (servo_set_thd)(SERVO2_ANGLE_MAX);   // 从未触发过时的默认停留 130°
                                let _ = (servo2_set_thd)(SERVO1_ANGLE_IDLE); // 舵机1 回 90°
                                idle_applied = true;
                            }
                        }
                    }
                })?;
            self.web.set_test_mode(test_mode);
            self.web.set_hardware_status(hw_status);
            info!("Hardware status and test-mode thread configured");
        }

        // Start Web server - serves the captive portal
        self.web.start()?;
        info!("Web server started");

        // AP 模式配网 TCP 1234（identify / config）；手机发完 config 可离开热点，STA 连上后在 WiFi 里持续发 binding，手机回信后完成绑定
        let (sta_start_tx, sta_start_rx) = mpsc::channel::<(String, String, Option<String>, Option<String>)>();
        let pending_config_done: PendingConfigDone = Arc::new(Mutex::new(None));
        let pending_bind_token: PendingBindToken = Arc::new(Mutex::new(None));
        let wifi_cmd_tx = self.wifi_cmd_tx.take().expect("wifi_cmd_tx");
        let sta_start_tx_for_loop = sta_start_tx.clone();
        let _wifi_cmd_tx_for_loop = wifi_cmd_tx.clone();
        let pending_config_done_loop = Arc::clone(&pending_config_done);
        let pending_bind_token_loop = Arc::clone(&pending_bind_token);
        let device_id = self.wifi.get_device_id();
        start_ap_tcp_listener(
            device_id.clone(),
            "1.0.0".to_string(),
            wifi_cmd_tx,
            None,
            pending_config_done,
            pending_bind_token,
        );
        let servo_for_sta = self.servo.clone();
        let servo2_for_sta = self.servo2.clone();
        let binding_state_clone = Arc::clone(&binding_state);
        let pair_state_clone = Arc::clone(&pair_state);
        let trigger_state_clone = Arc::clone(&trigger_state);
        let sync_running_clone = Arc::clone(&sync_running);
        let wifi_list_store_clone = Arc::clone(&wifi_list_store);
        let bind_token_store: BindTokenStore = Arc::new(Mutex::new(None));

        // Auto-start STA services only when this device is *already bound* (NVS).
        // Otherwise provisioning/add-device flow may be broken:
        // - provisioning needs STA services to broadcast `evt=binding` using the fresh `bind_token`
        // - if we start early with `bind_token=None` and set STA_SERVICES_STARTED, later provisioning spawn will be skipped
        let auto_bound = {
            match EspDefaultNvs::new(self._nvs.clone(), "bt_fy", true) {
                Ok(store) => store
                    .get_u8("binding.bound")
                    .ok()
                    .flatten()
                    .map(|v| v != 0)
                    .unwrap_or(false),
                Err(_) => false,
            }
        };

        if auto_bound {
            // Only mark STA_SERVICES_STARTED after we have a valid STA connection
            // (otherwise we could block the provisioning path forever).
            match self.wifi.execute(WifiCommand::GetStatus) {
                WifiResponse::Status(Some(sta)) => {
                    if !STA_SERVICES_STARTED.swap(true, Ordering::Relaxed) {
                        let did = self.wifi.get_device_id();
                        // Best-effort: bind_token is not required when binding_state is restored from NVS.
                        spawn_sta_services_on_connect(
                            did,
                            sta.ip.clone(),
                            Some(sta.ssid.clone()),
                            servo_for_sta.clone(),
                            servo2_for_sta.clone(),
                            Arc::clone(&binding_state_clone),
                            Arc::clone(&pair_state_clone),
                            Arc::clone(&trigger_state_clone),
                            Arc::clone(&sync_running_clone),
                            Arc::clone(&wifi_list_store_clone),
                            Arc::clone(&bind_token_store),
                            Arc::clone(&nvs_handle),
                        );
                        info!(
                            "Auto-started STA services on boot (sta.ip={}, sta.ssid={})",
                            sta.ip, sta.ssid
                        );
                    } else {
                        log::warn!("STA services already running (auto-start path)");
                    }
                }
                _ => {
                    info!("Skip auto-start STA services on boot: STA not connected yet");
                }
            }
        } else {
            info!("Skip auto-start STA services on boot: not bound in NVS");
        }

        std::thread::Builder::new()
            .name("sta-starter".into())
            .spawn(move || {
                while let Ok((did, sta_ip, bind_token, sta_ssid)) = sta_start_rx.recv() {
                    // Make provisioning token available to the already-running UDP broadcaster.
                    *bind_token_store.lock().unwrap() = bind_token;

                    if STA_SERVICES_STARTED.swap(true, Ordering::Relaxed) {
                        log::warn!("STA services already running, skip duplicate spawn");
                        continue;
                    }
                    spawn_sta_services_on_connect(
                        did,
                        sta_ip,
                        sta_ssid,
                        servo_for_sta.clone(),
                        servo2_for_sta.clone(),
                        Arc::clone(&binding_state_clone),
                        Arc::clone(&pair_state_clone),
                        Arc::clone(&trigger_state_clone),
                        Arc::clone(&sync_running_clone),
                        Arc::clone(&wifi_list_store_clone),
                        Arc::clone(&bind_token_store),
                        Arc::clone(&nvs_handle),
                    );
                }
            })?;

        // Print system status
        Self::print_status(&self);

        // Command loop: process WiFi commands from HTTP handlers and AP TCP
        let mut wifi = self.wifi;
        let wifi_cmd_rx = self.wifi_cmd_rx.take().expect("wifi_cmd_rx");
        info!("System is running.");
        loop {
            match wifi_cmd_rx.recv_timeout(Duration::from_secs(1)) {
                Ok((cmd, reply_tx)) => {
                    let response = wifi.execute(cmd);
                    // Connect 成功后：取走 pending_bind_token，启动 STA 服务（UDP 持续发 binding 直至收到手机回信）
                    if let WifiResponse::Connect(Ok(ref sta)) = response {
                        let did = wifi.get_device_id();
                        let bind_token = pending_bind_token_loop.lock().unwrap().take();
                        if let Some(ref t) = bind_token {
                            info!("Connect OK: bind_token passed to STA services (len={})", t.len());
                        } else {
                            info!("Connect OK: bind_token is None (pending_bind_token was empty)");
                        }
                        if let Some(tx) = pending_config_done_loop.lock().unwrap().take() {
                            let _ = tx.send((did.clone(), sta.ip.clone(), did.clone()));
                        }
                        let _ = sta_start_tx_for_loop.send((
                            did,
                            sta.ip.clone(),
                            bind_token,
                            Some(sta.ssid.clone()),
                        ));
                    }
                    let _ = reply_tx.send(response);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(())
    }

    /// Initialize ESP32 system components
    fn init_system() -> Result<(), Box<dyn std::error::Error>> {
        // Link ESP-IDF runtime patches
        esp_idf_svc::sys::link_patches();

        // Initialize logger
        EspLogger::initialize_default();

        Ok(())
    }

    /// Print system status information
    fn print_status(system: &Self) {
        info!("========================================");
        info!("Butterfly Captive Portal is Ready!");
        info!("========================================");
        info!("WiFi SSID: {} (no password)", system.wifi.ap_ssid());
        info!("IP Address: {}", AP_IP_ADDRESS);
        if ENABLE_DNS_CAPTIVE {
            info!("DNS Server: {}:53", AP_IP_ADDRESS);
        }
        info!("HTTP Server: http://{}", AP_IP_ADDRESS);

        if system.sensor.is_some() || system.servo.is_some() || system.servo2.is_some() {
            info!("Hardware: ENABLED");
            info!(
                "  - VL53L0X sensor: {}",
                if system.sensor.is_some() {
                    "READY"
                } else {
                    "NOT FOUND"
                }
            );
            info!(
                "  - Servo (GPIO3): {}",
                if system.servo.is_some() {
                    "READY"
                } else {
                    "NOT FOUND"
                }
            );
            info!(
                "  - Servo (GPIO4): {}",
                if system.servo2.is_some() {
                    "READY"
                } else {
                    "NOT FOUND"
                }
            );
            info!("API Endpoints:");
            info!("  - http://{}/api/status", AP_IP_ADDRESS);
            info!("  - http://{}/api/distance", AP_IP_ADDRESS);
            info!("  - http://{}/api/servo", AP_IP_ADDRESS);
            info!("  - http://{}/api/servo2", AP_IP_ADDRESS);
        } else {
            info!("Hardware: DISABLED (captive portal only)");
        }

        info!("========================================");
        info!("Connect to '{}' WiFi", system.wifi.ap_ssid());
        info!("Captive portal should auto-open!");
        info!("========================================");
    }
}
