//! NetworkShimMachine — bridges QiTech mutations to the winex_shim HTTP server
//! and polls live values from it every 200 ms.
//!
//! Uses only std::net::TcpStream so no extra crate dependency is needed.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc as mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use smol::channel::{Receiver, Sender, bounded};
use tracing::{info, warn};

use crate::{
    AsyncThreadMessage, Machine, MachineAct, MachineApi, MachineIdentification,
    MachineMessage, MachineValues, MACHINE_EXTRUDER_V2, VENDOR_QITECH,
    machine_identification::MachineIdentificationUnique,
};
use control_core::socketio::namespace::Namespace;

pub const NETWORK_SHIM_SERIAL: u16 = 0xB001;

#[derive(Default, Clone)]
struct LiveCache {
    rpm: f64,
    pressure: f64,
    temps: [f64; 4],
}

#[derive(Debug)]
pub struct NetworkShimMachine {
    uid: MachineIdentificationUnique,
    sender: Sender<MachineMessage>,
    receiver: Receiver<MachineMessage>,
    namespace: Option<Namespace>,
    cache: Arc<Mutex<LiveCache>>,
    mutation_tx: mpsc::Sender<Value>,
}

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
    // Body starts after the blank line separating headers from body
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
    // Drain response to allow server to finish writing
    let mut buf = [0u8; 256];
    let _ = stream.read(&mut buf);
}

// ---------------------------------------------------------------------------

impl NetworkShimMachine {
    pub fn new(base_url: String) -> Self {
        let (api_tx, api_rx) = bounded(64);
        let cache = Arc::new(Mutex::new(LiveCache::default()));
        let (mut_tx, mut_rx) = mpsc::channel::<Value>();

        // poller thread
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
                            }
                        }
                        None => warn!("NetworkShimMachine poller: request failed"),
                    }
                    std::thread::sleep(Duration::from_millis(200));
                })
                .expect("failed to spawn poller thread");
        }

        // forwarder thread
        {
            let url = base_url.clone();
            std::thread::Builder::new()
                .name("winex-shim-forwarder".into())
                .spawn(move || {
                    while let Ok(mutation) = mut_rx.recv() {
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
            namespace: None,
            cache,
            mutation_tx: mut_tx,
        }
    }

    pub fn uid(&self) -> MachineIdentificationUnique {
        self.uid.clone()
    }

    fn current_machine_values(&self) -> MachineValues {
        let c = self.cache.lock().unwrap().clone();
        MachineValues {
            state: serde_json::json!({ "mode": "unknown" }),
            live_values: serde_json::json!({
                "motor_status": { "rpm": c.rpm },
                "pressure": c.pressure,
                "temperatures": {
                    "front":  c.temps[0],
                    "middle": c.temps[1],
                    "back":   c.temps[2],
                    "nozzle": c.temps[3],
                }
            }),
        }
    }
}

impl MachineAct for NetworkShimMachine {
    fn act_machine_message(&mut self, msg: MachineMessage) {
        match msg {
            MachineMessage::SubscribeNamespace(ns) => self.namespace = Some(ns),
            MachineMessage::UnsubscribeNamespace => self.namespace = None,
            MachineMessage::HttpApiJsonRequest(value) => {
                let _ = self.mutation_tx.send(value);
            }
            MachineMessage::RequestValues(reply) => {
                let _ = reply.try_send(self.current_machine_values());
            }
        }
    }

    fn act(&mut self, _now: Instant) {
        while let Ok(msg) = self.receiver.try_recv() {
            self.act_machine_message(msg);
        }
    }
}

impl MachineApi for NetworkShimMachine {
    fn api_get_sender(&self) -> Sender<MachineMessage> {
        self.sender.clone()
    }

    fn api_mutate(&mut self, value: Value) -> anyhow::Result<()> {
        let _ = self.mutation_tx.send(value);
        Ok(())
    }

    fn api_event_namespace(&mut self) -> Option<Namespace> {
        self.namespace.take()
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
