use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::str::FromStr;
use std::sync::{mpsc, Mutex};
use std::thread;

use serde::{Deserialize, Serialize};

use event_derive::NetworkEvent;

use crate::context::Context;
use crate::event::{Event, NetworkEvent};
use crate::layer::Layer;

pub mod client {
    use super::*;

    #[derive(Debug)]
    pub struct IncomingNetworkLayer {
        remote: SocketAddr,
        socket: UdpSocket,
        receiver: Mutex<Option<mpsc::Receiver<Box<dyn NetworkEvent>>>>,
        thread: Mutex<Option<thread::JoinHandle<()>>>,
    }

    impl IncomingNetworkLayer {
        fn shutdown_thread(&self) {
            send_event(&self.socket, self.socket.local_addr().unwrap(), Box::new(ShutdownListenerEvent { })).unwrap();

            let mut guard = self.thread.lock().unwrap();
            if guard.is_some() {
                guard.take().unwrap().join().ok();
            }
            *self.receiver.lock().unwrap() = None;
        }
    }

    impl Layer for IncomingNetworkLayer {
        fn get_id(&self) -> String {
            "client::IncomingNetworkLayer".to_string()
        }

        fn on_add(&mut self, _: &Context) {
            log!("client::IncomingNetworkLayer added to the stack. Starting listening thread.");
            let (sender, receiver) = mpsc::channel();
            let socket = self.socket.try_clone().expect("Cannot clone socket");

            let thread = thread::spawn(move || {
                let mut buffer = [0u8; 1024];
                loop {
                    let result = receive_event(&socket, &mut buffer);
                    if result.is_err() {
                        log!(ERROR, "client::IncomingNetworkLayer listening thread: Could not receive event.");
                        continue;
                    }

                    let (address, event) = result.unwrap();

                    if event.as_any().is::<ShutdownListenerEvent>() {
                        log!("client::IncomingNetworkLayer listening thread received a shutdown event.");
                        break;
                    }

                    if address == socket.local_addr().unwrap() {
                        sender.send(event).unwrap();
                    }
                }
            });

            *self.receiver.lock().unwrap() = Some(receiver);
            *self.thread.lock().unwrap() = Some(thread);
        }

        fn on_remove(&mut self, _: &Context) {
            log!("client::IncomingNetworkLayer removed from the stack. Shutting down the listening thread.");
            self.shutdown_thread();
        }

        fn filter_gather_events(&mut self, _: &Context, incoming_events: Vec<Box<dyn Event>>) -> Vec<Box<dyn Event>> {
            let mut outgoing_events = incoming_events;

            let guard = self.receiver.lock().unwrap();

            if guard.is_none() {
                return outgoing_events;
            }

            loop {
                if let Ok(network_event) = guard.as_ref().unwrap().try_recv() {
                    outgoing_events.push(network_event.into());
                } else {
                    break;
                }
            }

            outgoing_events
        }
    }

    impl Drop for IncomingNetworkLayer {
        fn drop(&mut self) {
            self.shutdown_thread();
        }
    }

    #[derive(Debug)]
    pub struct OutgoingNetworkLayer {
        remote: SocketAddr,
        socket: UdpSocket,
    }

    impl Layer for OutgoingNetworkLayer {
        fn get_id(&self) -> String {
            "ClientOutgoingNetworkLayer".to_string()
        }

        fn on_add(&mut self, _: &Context) {
        }

        fn on_remove(&mut self, _: &Context) {
        }

        fn filter_gather_events(&mut self, _: &Context, incoming_events: Vec<Box<dyn Event>>) -> Vec<Box<dyn Event>> {
            let mut outgoing_events = Vec::new();

            for event in incoming_events {
                if event.is_network_event() {
                    let network_event = event.as_network_event().unwrap();

                    if send_event(&self.socket, self.remote, network_event).is_err() {
                        log!(ERROR, "client::OutgoingNetworkLayer::filter_gather_events: Could not send event.");
                    }
                } else {
                    outgoing_events.push(event);
                }
            }

            outgoing_events
        }
    }

    pub fn create_network_layer_pair(bind: &str, connect: &str) -> Result<(IncomingNetworkLayer, OutgoingNetworkLayer), String> {
        let socket = UdpSocket::bind(bind).map_err(|_| "Cannot bind to address")?;
        let remote = SocketAddr::from_str(connect).unwrap();

        Ok((
            IncomingNetworkLayer {
                remote: remote,
                socket: socket.try_clone().map_err(|_| "Cannot clone socket")?,
                receiver: Mutex::new(None),
                thread: Mutex::new(None),
            },
            OutgoingNetworkLayer {
                remote: remote,
                socket: socket,
            },
        ))
    }
}

#[derive(Clone, Debug, Deserialize, NetworkEvent, Serialize)]
struct ShutdownListenerEvent { }

fn send_event(socket: &UdpSocket, destination: SocketAddr, event: Box<dyn NetworkEvent>) -> io::Result<usize> {
    let buffer = serde_cbor::to_vec(&event).map_err(|_| io::Error::new(io::ErrorKind::Other, "Could not serialize event"))?;
    socket.send_to(&buffer, destination)
}

fn receive_event(socket: &UdpSocket, buffer: &mut [u8]) -> io::Result<(SocketAddr, Box<dyn NetworkEvent>)> {
    let (size, address) = socket.recv_from(buffer)?;
    let event = serde_cbor::from_slice(&buffer[..size]).map_err(|_| io::Error::new(io::ErrorKind::Other, "Could not deserialize packet"))?;
    Ok((address, event))
}