//! NetworkShimMachine — bridges QiTech mutations to the winex_shim HTTP server
//! and polls live values from it every 200 ms.
//!
//! Uses only std::net::TcpStream so no extra crate dependency is needed.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use smol::channel::{Receiver, Sender, bounded};
use tracing::{info, warn};

use crate::{
    AsyncThreadMessage, Machine, MachineAct, MachineApi,
    MachineMessage, MachineValues, MACHINE_EXTRUDER_V2, VENDOR_QITECH,
    extruder1::{
        ExtruderV2Mode,
        api::{
            ExtruderSettingsState, ExtruderV2Events, ExtruderV2Namespace, HeatingState,
            HeatingStates, InverterStatusState, LiveValuesEvent, ModeState, MotorStatusValues,
            PidAutoTuneState, PidSettings, PidSettingsStates, PressureState, RegulationState,
            RotationState, ScrewState, StateEvent, TemperaturePid, TemperaturePidStates,
        },
    },
    machine_identification::{MachineIdentification, MachineIdentificationUnique},
};
use control_core::socketio::namespace::NamespaceCacheingLogic;

pub const NETWORK_SHIM_SERIAL: u16 = 0xB001;
const PRESSURE_CONTROL_INTERVAL_S: u64 = 1;
const PRESSURE_CONTROL_DEADBAND_BAR: f64 = 0.5;
const PRESSURE_CONTROL_KP_RPM_PER_BAR: f64 = 0.2;
const PRESSURE_CONTROL_KI_RPM_PER_BAR_S: f64 = 0.02;
const PRESSURE_CONTROL_MAX_RPM: f64 = 44.0;

// ---------------------------------------------------------------------------
// Minimal HTTP helpers using only stdlib — no TLS needed (local HTTP only)
// ---------------------------------------------------------------------------

fn parse_http_url(url: &str) -> Option<(String, String)> {
    let without_scheme = url.strip_prefix("http://")?;
    let slash_pos = without_scheme.find('/').unwrap_or(without_scheme.len());
    let host_port = without_scheme[..slash_pos].to_string();
    let path = if slash_pos < without_scheme.len() {
        without_scheme[slash_pos..].to_string()
    } else {
        "/".to_string()
    };
    Some((host_port, path))
}

fn http_get(url: &str, timeout: Duration) -> Option<String> {
    let (host_port, path) = parse_http_url(url)?;
    let mut stream = TcpStream::connect(&host_port).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;
    let request = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, host_port
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).ok()?;
    let response = String::from_utf8_lossy(&response).into_owned();
    response.find("\r\n\r\n").map(|i| response[i + 4..].to_string())
}

fn http_post_json(url: &str, body: &str, timeout: Duration) {
    let Some((host_port, path)) = parse_http_url(url) else {
        return;
    };
    let Ok(mut stream) = TcpStream::connect(&host_port) else {
        return;
    };
    let _ = stream.set_write_timeout(Some(timeout));
    let _ = stream.set_read_timeout(Some(timeout));
    let request = format!(
        "POST {} HTTP/1.0\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host_port, body.len(), body
    );
    let _ = stream.write_all(request.as_bytes());
    let mut buf = [0u8; 256];
    let _ = stream.read(&mut buf);
}

// ---------------------------------------------------------------------------
// Live cache shared between the poll thread and the RT loop
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
struct LiveCache {
    rpm: f64,
    pressure: f64,
    /// front, middle, back, nozzle
    temps: [f64; 4],
    drive_active: bool,
    tempering_active: bool,
}

// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct NetworkShimMachine {
    uid: MachineIdentificationUnique,
    sender: Sender<MachineMessage>,
    receiver: Receiver<MachineMessage>,
    namespace: ExtruderV2Namespace,
    cache: Arc<Mutex<LiveCache>>,
    mutation_tx: Sender<Value>,

    // State tracked by this machine
    mode: ExtruderV2Mode,
    uses_rpm: bool,
    target_pressure: f64,
    target_rpm: f64,
    target_temps: [f64; 4],
    pressure_integral: f64,
    last_pressure_control: Instant,
    emitted_default_state: bool,
    last_emit: Instant,
}

impl NetworkShimMachine {
    pub fn new(base_url: String) -> Self {
        let (api_tx, api_rx) = bounded(64);
        let cache = Arc::new(Mutex::new(LiveCache::default()));
        let (mut_tx, mut_rx) = bounded::<Value>(64);

        // poller thread — reads live values from WinExShim every 200ms
        {
            let cache = cache.clone();
            let url = base_url.clone();
            std::thread::Builder::new()
                .name("winex-shim-poller".into())
                .spawn(move || loop {
                    let endpoint = format!("{url}/api/v1/live_values");
                    match http_get(&endpoint, Duration::from_millis(600)) {
                        Some(body) => {
                            if let Ok(json) = serde_json::from_str::<Value>(&body) {
                                let mut c = cache.lock().unwrap();
                                if let Some(v) =
                                    json.pointer("/motor_status/rpm").and_then(Value::as_f64)
                                {
                                    c.rpm = v;
                                }
                                if let Some(v) = json.get("pressure").and_then(Value::as_f64) {
                                    c.pressure = v;
                                }
                                if let Some(t) = json.get("temperatures") {
                                    for (i, key) in
                                        ["front", "middle", "back", "nozzle"].iter().enumerate()
                                    {
                                        if let Some(v) = t.get(*key).and_then(Value::as_f64) {
                                            c.temps[i] = v;
                                        }
                                    }
                                }
                                if let Some(v) = json
                                    .pointer("/motor_status/drive_active")
                                    .and_then(Value::as_bool)
                                {
                                    c.drive_active = v;
                                }
                                if let Some(v) =
                                    json.get("tempering_enabled").and_then(Value::as_bool)
                                {
                                    c.tempering_active = v;
                                }
                            }
                        }
                        None => warn!("NetworkShimMachine poller: request failed"),
                    }
                    std::thread::sleep(Duration::from_millis(200));
                })
                .expect("failed to spawn poller thread");
        }

        // forwarder thread — sends mutations to WinExShim
        {
            let url = base_url.clone();
            std::thread::Builder::new()
                .name("winex-shim-forwarder".into())
                .spawn(move || {
                    while let Ok(mutation) = mut_rx.recv_blocking() {
                        let body = if mutation.is_array() {
                            mutation
                        } else {
                            serde_json::json!([mutation])
                        };
                        let endpoint = format!("{url}/api/v1/mutations");
                        http_post_json(&endpoint, &body.to_string(), Duration::from_millis(2000));
                    }
                })
                .expect("failed to spawn forwarder thread");
        }

        info!("NetworkShimMachine: registered for {base_url}");

        Self {
            uid: MachineIdentificationUnique {
                machine_identification: MachineIdentification {
                    vendor: VENDOR_QITECH,
                    machine: MACHINE_EXTRUDER_V2,
                },
                serial: NETWORK_SHIM_SERIAL,
            },
            sender: api_tx,
            receiver: api_rx,
            namespace: ExtruderV2Namespace { namespace: None },
            cache,
            mutation_tx: mut_tx,
            mode: ExtruderV2Mode::Standby,
            uses_rpm: true,
            target_pressure: 0.0,
            target_rpm: 0.0,
            target_temps: [160.0, 160.0, 160.0, 160.0],
            pressure_integral: 0.0,
            last_pressure_control: Instant::now(),
            emitted_default_state: false,
            last_emit: Instant::now(),
        }
    }

    pub fn uid(&self) -> MachineIdentificationUnique {
        self.uid.clone()
    }

    fn build_live_values(&self) -> LiveValuesEvent {
        let c = self.cache.lock().unwrap().clone();
        LiveValuesEvent {
            motor_status: MotorStatusValues {
                screw_rpm: c.rpm,
                frequency: 0.0,
                voltage: 0.0,
                current: 0.0,
                power: 0.0,
            },
            pressure: c.pressure,
            front_temperature: c.temps[0],
            middle_temperature: c.temps[1],
            back_temperature: c.temps[2],
            nozzle_temperature: c.temps[3],
            front_power: 0.0,
            middle_power: 0.0,
            back_power: 0.0,
            nozzle_power: 0.0,
            combined_power: 0.0,
            total_energy_kwh: 0.0,
        }
    }

    fn build_state_event(&mut self) -> StateEvent {
        let is_default = !std::mem::replace(&mut self.emitted_default_state, true);
        StateEvent {
            is_default_state: is_default,
            rotation_state: RotationState { forward: true },
            mode_state: ModeState { mode: self.mode.clone() },
            regulation_state: RegulationState { uses_rpm: self.uses_rpm },
            pressure_state: PressureState {
                target_bar: self.target_pressure,
                wiring_error: false,
            },
            screw_state: ScrewState { target_rpm: self.target_rpm },
            heating_states: HeatingStates {
                front:  HeatingState { target_temperature: self.target_temps[0], wiring_error: false },
                middle: HeatingState { target_temperature: self.target_temps[1], wiring_error: false },
                back:   HeatingState { target_temperature: self.target_temps[2], wiring_error: false },
                nozzle: HeatingState { target_temperature: self.target_temps[3], wiring_error: false },
            },
            extruder_settings_state: ExtruderSettingsState {
                pressure_limit: 200.0,
                pressure_limit_enabled: false,
                nozzle_temperature_target_enabled: true,
            },
            inverter_status_state: InverterStatusState {
                running: self.mode == ExtruderV2Mode::Extrude,
                forward_running: self.mode == ExtruderV2Mode::Extrude,
                reverse_running: false,
                up_to_frequency: false,
                overload_warning: false,
                no_function: false,
                output_frequency_detection: false,
                abc_fault: false,
                fault_occurence: false,
            },
            pid_settings: PidSettingsStates {
                temperature: TemperaturePidStates {
                    front:  TemperaturePid { ki: 0.0, kp: 1.0, kd: 0.0, zone: "front".into() },
                    middle: TemperaturePid { ki: 0.0, kp: 1.0, kd: 0.0, zone: "middle".into() },
                    back:   TemperaturePid { ki: 0.0, kp: 1.0, kd: 0.0, zone: "back".into() },
                    nozzle: TemperaturePid { ki: 0.0, kp: 1.0, kd: 0.0, zone: "nozzle".into() },
                },
                pressure: PidSettings { ki: 0.0, kp: 1.0, kd: 0.0 },
            },
            pid_autotune_state: PidAutoTuneState::default(),
        }
    }

    fn emit_state(&mut self) {
        use control_core::socketio::event::BuildEvent;
        let event = self.build_state_event().build();
        self.namespace.emit(ExtruderV2Events::State(event));
    }

    fn emit_live_values(&mut self) {
        use control_core::socketio::event::BuildEvent;
        let event = self.build_live_values().build();
        self.namespace.emit(ExtruderV2Events::LiveValues(event));
    }

    fn infer_mode_from_cache(&self) -> ExtruderV2Mode {
        let c = self.cache.lock().unwrap().clone();
        if c.drive_active {
            ExtruderV2Mode::Extrude
        } else if c.tempering_active {
            ExtruderV2Mode::Heat
        } else {
            ExtruderV2Mode::Standby
        }
    }

    fn update_pressure_regulation(&mut self, now: Instant) {
        if self.uses_rpm
            || self.mode != ExtruderV2Mode::Extrude
            || self.target_pressure <= 0.0
            || self.target_rpm <= 0.0
            || now.duration_since(self.last_pressure_control)
                < Duration::from_secs(PRESSURE_CONTROL_INTERVAL_S)
        {
            return;
        }

        let elapsed_s = now
            .duration_since(self.last_pressure_control)
            .as_secs_f64()
            .max(0.001);
        self.last_pressure_control = now;

        let pressure = self.cache.lock().unwrap().pressure;
        let error = self.target_pressure - pressure;
        if error.abs() <= PRESSURE_CONTROL_DEADBAND_BAR {
            return;
        }

        self.pressure_integral =
            (self.pressure_integral + error * elapsed_s).clamp(-100.0, 100.0);

        let rpm_delta =
            error * PRESSURE_CONTROL_KP_RPM_PER_BAR
                + self.pressure_integral * PRESSURE_CONTROL_KI_RPM_PER_BAR_S;
        let next_rpm = (self.target_rpm + rpm_delta).clamp(0.0, PRESSURE_CONTROL_MAX_RPM);

        if (next_rpm - self.target_rpm).abs() < 0.1 {
            return;
        }

        self.target_rpm = next_rpm;
        let _ = self
            .mutation_tx
            .try_send(serde_json::json!({"SetInverterTargetRpm": next_rpm}));
        self.emit_state();
    }

    fn current_machine_values(&mut self) -> MachineValues {
        let state = self.build_state_event();
        let live = self.build_live_values();
        MachineValues {
            state: serde_json::to_value(&state).unwrap_or_default(),
            live_values: serde_json::to_value(&live).unwrap_or_default(),
        }
    }
}

impl MachineAct for NetworkShimMachine {
    fn act(&mut self, now: Instant) {
        while let Ok(msg) = self.receiver.try_recv() {
            self.act_machine_message(msg);
        }

        // Auto-detect mode from live values (only if we're in Standby and might be wrong)
        if self.mode == ExtruderV2Mode::Standby {
            let inferred = self.infer_mode_from_cache();
            if inferred != ExtruderV2Mode::Standby {
                self.mode = inferred;
                self.emit_state();
            }
        }

        self.update_pressure_regulation(now);

        // Emit live values and state at ~30fps
        if now.duration_since(self.last_emit) >= Duration::from_millis(33) {
            self.last_emit = now;
            self.emit_live_values();
        }
    }

    fn act_machine_message(&mut self, msg: MachineMessage) {
        match msg {
            MachineMessage::SubscribeNamespace(ns) => {
                self.namespace.namespace = Some(ns);
                self.emitted_default_state = false;
                self.emit_state();
            }
            MachineMessage::UnsubscribeNamespace => {
                self.namespace.namespace = None;
            }
            MachineMessage::HttpApiJsonRequest(value) => {
                // Parse mutation and update local state before forwarding
                if let Some(obj) = value.as_object() {
                    if let Some(mode_val) = obj.get("SetExtruderMode") {
                        if let Some(mode_str) = mode_val.as_str() {
                            self.mode = match mode_str {
                                "Standby" => ExtruderV2Mode::Standby,
                                "Heat" => ExtruderV2Mode::Heat,
                                "Extrude" => ExtruderV2Mode::Extrude,
                                _ => self.mode.clone(),
                            };
                            self.emit_state();
                        }
                    } else if let Some(rpm_val) = obj.get("SetInverterTargetRpm") {
                        if let Some(rpm) = rpm_val.as_f64() {
                            self.target_rpm = rpm;
                            self.pressure_integral = 0.0;
                            self.emit_state();
                        }
                    } else if let Some(pressure_val) = obj.get("SetInverterTargetPressure") {
                        if let Some(pressure) = pressure_val.as_f64() {
                            self.target_pressure = pressure;
                            self.pressure_integral = 0.0;
                            self.emit_state();
                        }
                    } else if let Some(regulation_val) = obj.get("SetInverterRegulation") {
                        if let Some(uses_rpm) = regulation_val.as_bool() {
                            self.uses_rpm = uses_rpm;
                            self.pressure_integral = 0.0;
                            self.last_pressure_control = Instant::now();
                            self.emit_state();
                        }
                    } else if let Some(t) = obj.get("SetFrontHeatingTargetTemperature") {
                        if let Some(v) = t.as_f64() { self.target_temps[0] = v; self.emit_state(); }
                    } else if let Some(t) = obj.get("SetMiddleHeatingTemperature").or(obj.get("SetMiddleHeatingTargetTemperature")) {
                        if let Some(v) = t.as_f64() { self.target_temps[1] = v; self.emit_state(); }
                    } else if let Some(t) = obj.get("SetBackHeatingTargetTemperature") {
                        if let Some(v) = t.as_f64() { self.target_temps[2] = v; self.emit_state(); }
                    } else if let Some(t) = obj.get("SetNozzleHeatingTemperature").or(obj.get("SetNozzleHeatingTargetTemperature")) {
                        if let Some(v) = t.as_f64() { self.target_temps[3] = v; self.emit_state(); }
                    }
                }
                let _ = self.mutation_tx.try_send(value);
            }
            MachineMessage::RequestValues(reply) => {
                let vals = self.current_machine_values();
                let _ = reply.send_blocking(vals);
            }
        }
    }
}

impl MachineApi for NetworkShimMachine {
    fn api_get_sender(&self) -> Sender<MachineMessage> {
        self.sender.clone()
    }

    fn api_mutate(&mut self, value: Value) -> anyhow::Result<()> {
        let _ = self.mutation_tx.try_send(value);
        Ok(())
    }

    fn api_event_namespace(&mut self) -> Option<control_core::socketio::namespace::Namespace> {
        self.namespace.namespace.clone()
    }
}

impl Machine for NetworkShimMachine {
    fn get_machine_identification_unique(&self) -> MachineIdentificationUnique {
        self.uid.clone()
    }

    fn get_main_sender(&self) -> Option<Sender<AsyncThreadMessage>> {
        None
    }
}
