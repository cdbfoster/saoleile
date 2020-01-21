use crate::layer::manager::LayerManager;

pub struct Context {
    pub layer_manager: LayerManager,
}

impl Context {
    pub fn shutdown(&self) {
        self.layer_manager.shutdown();
    }
}