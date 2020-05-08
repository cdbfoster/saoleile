use crate::layer::manager::LayerManager;
use crate::network::NetworkInterface;

#[derive(Debug)]
pub struct Context {
    pub layer_manager: LayerManager,
    pub network_interface: NetworkInterface,
}

impl Context {
    pub fn shutdown(&self) {
        self.layer_manager.shutdown();
    }
}