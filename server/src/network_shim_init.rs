//! Startup initializer for the NetworkShimMachine.
//! Called from main() if the WINEXT_SHIM_URL environment variable is set.

use std::sync::Arc;

use machines::network_shim::NetworkShimMachine;

use crate::{
    app_state::{HotThreadMessage, SharedState},
    socketio::main_namespace::machines_event::MachineObj,
};

pub async fn init_network_shim(app_state: Arc<SharedState>, base_url: String) {
    tracing::info!("NetworkShimMachine: initializing with URL {base_url}");

    let machine = NetworkShimMachine::new(base_url, Some(app_state.main_channel.clone()));
    let uid = machine.uid();
    let machine: Box<dyn machines::Machine> = Box::new(machine);

    app_state
        .add_machines_if_not_exists(vec![MachineObj {
            machine_identification_unique: uid.clone(),
            error: None,
        }])
        .await;

    app_state
        .api_machines
        .lock()
        .await
        .insert(uid, machine.api_get_sender());

    let _ = app_state
        .rt_machine_creation_channel
        .send(HotThreadMessage::AddMachines(vec![machine]))
        .await;

    app_state.clone().send_machines_event().await;

    tracing::info!("NetworkShimMachine: registered as extruder_v2 (serial 0xB001)");
}
