#[macro_use]
pub mod log;

// XXX Fix visibility
pub mod component;
pub mod context;
pub mod event;
pub mod layer;
pub mod network;
pub mod scene;
pub mod timer;
pub mod util;

#[cfg(target_os = "android")]
pub fn initialize_app() -> u64 {
    log!("Hello world!");

    /*let context = std::sync::Arc::new(context::Context {
        layer_manager: layer::manager::LayerManager::new(),
    });

    use event::EventDispatcher;
    context.layer_manager.dispatch_event(Box::new(event::core::ContextEvent {
        context: context.clone(),
    }));

    let (incoming, mut outgoing) = layer::network::client::create_network_layer_pair("127.0.0.1:4455", "127.0.0.1:4455").unwrap();

    context.layer_manager.dispatch_event(Box::new(event::layer::AddLayerEvent {
        push: event::layer::LayerPosition::Top,
        pin: None,
        layer: Box::new(incoming),
    }));

    std::thread::sleep(std::time::Duration::from_millis(1000));

    use layer::Layer;
    outgoing.filter_gather_events(&context, vec![
        Box::new(event::DummyEvent { }),
        Box::new(event::DummyEvent { }),
    ]);

    std::thread::sleep(std::time::Duration::from_millis(1000));

    context.shutdown();

    log!("Exiting");*/

    0
}