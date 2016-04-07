use ws::{listen as ws_listen, Sender, Handler, Message as WSMessage, Result, Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde_json;
use serde::{Deserialize, Serialize};
use rmp_serde::Deserializer as RMPDeserializer;
use rmp_serde::Serializer;
use rmp::Marker;
use rmp::encode::{ValueWriteError, write_map_len, write_str};
use rmp_serde::encode::VariantWriter;
use std::io::{Cursor, Write};
use messages::Message;

struct Client {
    sender: Mutex<Sender>
}

struct Router<'a> {
    subscriptions : Mutex<HashMap<String, &'a ConnectionHandler<'a>>>
}

struct ConnectionHandler<'a> {
    state: ConnectionState,
    sender: Mutex<Sender>,
    router: Arc<Router<'a>>
}

enum ConnectionState {
    Initializing,
    Connected,
    ShuttingDown,
    Disconnected
}

pub fn listen(url: &str) {
    let router = Arc::new(Router {
        subscriptions: Mutex::new(HashMap::new())
    });
    ws_listen(url, |sender| {
        ConnectionHandler {
            state: ConnectionState::Initializing,
            sender: Mutex::new(sender),
            router: router.clone()
        }
    }).unwrap()
    // Create a Router
    // use ARC to give everyone a reference to it
    // Each connection has direct connection to a client, as well as to a router
}

impl<'a> ConnectionHandler<'a> {
    fn handle_message(&mut self, message: Message) -> Result<()> {
        Ok(())
    }
}

impl<'a> Handler for ConnectionHandler<'a> {
    fn on_message(&mut self, msg: WSMessage) -> Result<()> {
        let message = try!(match msg {
            WSMessage::Text(payload) => {
                match serde_json::from_str(&payload) {
                    Ok(message) => Ok(message),
                    Err(e) => Err(Error::new(ErrorKind::Custom(Box::new(e)), "unable to parse message"))
                }
            },
            WSMessage::Binary(payload) => {
                let mut de = RMPDeserializer::new(Cursor::new(payload));
                match Deserialize::deserialize(&mut de) {
                    Ok(message) => {
                        Ok(message)
                    },
                    Err(e) => {
                        Err(Error::new(ErrorKind::Custom(Box::new(e)), "unable to parse message"))
                    }
                }
            }
        });

        self.handle_message(message)
    }
}
