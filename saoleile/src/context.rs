use std::net::SocketAddr;
use std::sync::Mutex;

use crate::layer::manager::LayerManager;
use crate::network::NetworkInterface;
use crate::scene::SceneManager;
use crate::timer::TimerManager;

#[derive(Debug)]
pub struct Context {
    pub layer_manager: LayerManager,
    pub scene_manager: SceneManager,
    pub timer_manager: TimerManager,
    pub network_interface: NetworkInterface,

    running: Mutex<bool>,
}

impl Context {
    pub fn new(network_bind_address: SocketAddr) -> Self {
        Self {
            layer_manager: LayerManager::new(),
            scene_manager: SceneManager::new(),
            timer_manager: TimerManager::new(),
            network_interface: NetworkInterface::new(network_bind_address),
            running: Mutex::new(true),
        }
    }

    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    pub fn shutdown(&self) {
        let mut running = self.running.lock().unwrap();

        if *running {
            self.layer_manager.shutdown();
            self.scene_manager.shutdown();
            self.timer_manager.shutdown();
            self.network_interface.shutdown();
            *running = false;
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        self.shutdown();
    }
}