//! NetworkShimMachine — bridges QiTech mutations to the winex_shim HTTP server
//! and polls live values from it every 200 ms.
//!
//! Does NOT perform blocking I/O on the RT thread. Two background threads handle
//! HTTP so that act() stays non-blocking:
//!   • poller thread  — GET /api/v1/live_values every 200 ms, writes to Arc<Mutex>
//!   • forwarder thread — reads from mpsc queue, POST /api/v1/mutations

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

/// Assigned serial — must be unique on the QiTech bus.
/// 0xB001 = "Brabender shim #1"
pub const NETWORK_SHIM_SERIAL: u16 = 0xB001;

// ---------------------------------------------------------------------------
// Shared cache written by the poller thread, read by act()
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
struct LiveCache {
    rpm: f64,
    pressure: f64,
    temps: [f64; 4], // [front, middle, back, nozzle]
}

// ---------------------------------------------------------------------------
// Machine struct
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct NetworkShimMachine {
    uid: MachineIdentificationUnique,
    // smol async channel — used by the QiTech API layer
    sender: Sender<MachineMessage>,
    receiver: Receiver<MachineMessage>,
    namespace: Option<Namespace>,
    // shared live-value cache updated by the poller thread
    cache: Arc<Mutex<LiveCache>>,
    // queue to the forwarder thread (non-blocking send from act())
    mutation_tx: mpsc::Sender<Value>,
}

impl NetworkShimMachine {
    pub fn new(base_url: String) -> Self {
        let (api_tx, api_rx) = bounded(64);
        let cache = Arc::new(Mutex::new(LiveCache::default()));
        let (mut_tx, mut_rx) = mpsc::channel::<Value>();

        // --- poller thread ---
        {
            let cache = cache.clone();
            let url = base_url.clone();
            std::thread::Builder::new()
                .name("winex-shim-poller".into())
                .spawn(move || {
                    loop {
                        let endpoint = format!("{url}/api/v1/live_values");
                        match ureq::get(&endpoint)
                            .timeout(Duration::from_millis(600))
                            .call()
                        {
                            Ok(resp) => {
                                if let Ok(json) = resp.into_json::<Value>() {
                                    let mut c = cache.lock().unwrap();
                                    if let Some(v) =
                                        json.pointer("/motor_status/rpm").and_then(Value::as_f64)
                                    {
                                        c.rpm = v;
                                    }
                                    if let Some(v) =
                                        json.get("pressure").and_then(Value::as_f64)
                                    {
                                        c.pressure = v;
                                    }
                                    if let Some(t) = json.get("temperatures") {
                                        for (i, key) in
                                            ["front", "middle", "back", "nozzle"].iter().enumerate()
                                        {
                                            if let Some(v) =
                                                t.get(*key).and_then(Value::as_f64)
                                            {
                                                c.temps[i] = v;
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => warn!("NetworkShimMachine poller: {e}"),
                        }
                        std::thread::sleep(Duration::from_millis(200));
                    }
                })
                .expect("failed to spawn poller thread");
        }

        // --- forwarder thread ---
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
                        if let Err(e) = ureq::post(&endpoint)
                            .timeout(Duration::from_millis(2000))
                            .set("Content-Type", "application/json")
                            .send_string(&body.to_string())
                        {
                            warn!("NetworkShimMachine forwarder: {e}");
                        }
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

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl MachineAct for NetworkShimMachine {
    fn act_machine_message(&mut self, msg: MachineMessage) {
        match msg {
            MachineMessage::SubscribeNamespace(ns) => self.namespace = Some(ns),
            MachineMessage::UnsubscribeNamespace => self.namespace = None,
            // Forward mutation to shim via non-blocking queue
            MachineMessage::HttpApiJsonRequest(value) => {
                let _ = self.mutation_tx.send(value);
            }
            // REST GET endpoint — reply with latest cached values
            MachineMessage::RequestValues(reply) => {
                let _ = reply.try_send(self.current_machine_values());
            }
        }
    }

    fn act(&mut self, _now: Instant) {
        // Drain all pending API messages — never blocks
        while let Ok(msg) = self.receiver.try_recv() {
            self.act_machine_message(msg);
        }
        // Live-value cache is updated by the poller thread in the background.
        // No I/O here — act() stays non-blocking.
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
