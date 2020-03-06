use crate::layer::manager::LayerManager;

#[derive(Debug)]
pub struct Context {
    pub layer_manager: LayerManager,
}

impl Context {
    pub fn shutdown(&self) {
        self.layer_manager.shutdown();
    }
}