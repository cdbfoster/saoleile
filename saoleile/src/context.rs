use crate::layer::manager::LayerManager;
use crate::network::NetworkInterface;
use crate::scene::SceneManager;

#[derive(Debug)]
pub struct Context {
    pub layer_manager: LayerManager,
    pub scene_manager: SceneManager,
    pub network_interface: NetworkInterface,
}

impl Context {
    pub fn shutdown(&self) {
        self.layer_manager.shutdown();
    }
}