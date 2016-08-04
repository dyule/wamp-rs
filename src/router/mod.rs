pub mod patterns;
//#[cfg(test)]
mod test;

use ws::{listen as ws_listen, Sender, Handler, Message as WSMessage, Error as WSError, ErrorKind as WSErrorKind, Result as WSResult, Request, Response, CloseCode};
use std::sync::{Arc, Mutex};

use std::collections::{HashMap};
use serde_json;
use serde::{Deserialize, Serialize};
use rmp_serde::Deserializer as RMPDeserializer;
use rmp_serde::Serializer;
use utils::StructMapWriter;
use std::io::Cursor;
use messages::{Message, URI, HelloDetails, WelcomeDetails, RouterRoles, SubscribeOptions, PublishOptions, EventDetails, ErrorDetails, ErrorType, Reason};
use ::{List, Dict, ID, MatchingPolicy, WampResult, Error, ErrorKind};
use std::marker::Sync;
use rand::{thread_rng};
use rand::distributions::{Range, IndependentSample};
use router::patterns::PatternNode;


struct SubscriptionManager {
    subscriptions : PatternNode<Arc<Mutex<ConnectionInfo>>>,
    subscription_ids_to_uris: HashMap<u64, (String, bool)>
}

struct Realm {
    subscription_manager: SubscriptionManager,
    connections: Vec<Arc<Mutex<ConnectionInfo>>>
}

pub struct Router {
    info: Arc<RouterInfo>
}

struct RouterInfo {
    realms: Mutex<HashMap<String, Arc<Mutex<Realm>>>>,
}

struct ConnectionHandler {
    info: Arc<Mutex<ConnectionInfo>>,
    router: Arc<RouterInfo>,
    realm: Option<Arc<Mutex<Realm>>>,
    subscribed_topics: Vec<ID>
}

pub struct ConnectionInfo {
    state: ConnectionState,
    sender: Sender,
    protocol: String,
    id: u64
}

#[derive(Clone)]
enum ConnectionState {
    Initializing,
    Connected,
    ShuttingDown,
    Disconnected
}

static WAMP_JSON:&'static str = "wamp.2.json";
static WAMP_MSGPACK:&'static str = "wamp.2.msgpack";

fn random_id() -> u64 {
    let mut rng = thread_rng();
    let between = Range::new(0, 1u64.rotate_left(56) - 1);
    between.ind_sample(&mut rng)
}

fn send_message(info: &Arc<Mutex<ConnectionInfo>>, message: &Message) -> WampResult<()> {
    let info = info.lock().unwrap();

    debug!("Sending message {:?} via {}", message, info.protocol);
    let send_result = if info.protocol == WAMP_JSON {
        send_message_json(&info.sender, message)
    } else {
        send_message_msgpack(&info.sender, message)
    };
    match send_result {
        Ok(()) => Ok(()),
        Err(e) => Err(Error::new(ErrorKind::WSError(e)))
    }
}

fn send_message_json(sender: &Sender, message: &Message) -> WSResult<()> {
    // Send the message
    sender.send(WSMessage::Text(serde_json::to_string(message).unwrap()))

}

fn send_message_msgpack(sender: &Sender, message: &Message) -> WSResult<()> {

    // Send the message
    let mut buf: Vec<u8> = Vec::new();
    message.serialize(&mut Serializer::with(&mut buf, StructMapWriter)).unwrap();
    sender.send(WSMessage::Binary(buf))

}

unsafe impl Sync for Router {}

impl Router {
    #[inline]
    pub fn new() -> Router {
        Router{
            info: Arc::new(RouterInfo {
                realms: Mutex::new(HashMap::new()),
            })
        }
    }

    pub fn listen(&self, url: &str) {

        ws_listen(url, |sender| {
            ConnectionHandler {
                info: Arc::new(Mutex::new(ConnectionInfo{
                    state: ConnectionState::Initializing,
                    sender: sender,
                    protocol: String::new(),
                    id: random_id()
                })),
                subscribed_topics: Vec::new(),
                realm: None,
                router: self.info.clone()
            }
        }).unwrap()
    }

    pub fn add_realm(&mut self, realm: &str) {
        let mut realms = self.info.realms.lock().unwrap();
        if realms.contains_key(realm) {
            return
        }
        realms.insert(realm.to_string(), Arc::new(Mutex::new(Realm {
            connections: Vec::new(),
            subscription_manager: SubscriptionManager {
                subscriptions: PatternNode::new(),
                subscription_ids_to_uris: HashMap::new()
            }
        })));
        debug!("Added realm {}", realm);
    }

    pub fn shutdown(&self) {

        for realm in self.info.realms.lock().unwrap().values() {
            for connection in realm.lock().unwrap().connections.iter() {
                let connection = connection.lock().unwrap();
                connection.sender.shutdown().ok();
            }
        }
    }
}



impl ConnectionHandler{
    fn handle_message(&mut self, message: Message) -> WampResult<()> {
        debug!("Recieved message {:?}", message);
        match message {
            Message::Hello(realm, details) => {
                self.handle_hello(realm, details)
            },
            Message::Subscribe(request_id, options, topic) => {
                self.handle_subscribe(request_id,  options, topic)
            },
            Message::Publish(request_id, options, topic, args, kwargs) => {
                self.handle_publish(request_id, options, topic, args, kwargs)
            },
            Message::Unsubscribe(request_id, topic_id) => {
                self.handle_unsubscribe(request_id, topic_id)
            },
            Message::Goodbye(details, reason) => {
                self.handle_goodbye(details, reason)
            }
            t => {
                Err(Error::new(ErrorKind::InvalidMessageType(t)))
            }
        }
    }

    fn handle_hello(&mut self, realm: URI, _details: HelloDetails) -> WampResult<()> {
        debug!("Responding to hello message (realm: {:?})", realm);
        let id = {
            let mut info = self.info.lock().unwrap();
            info.state = ConnectionState::Connected;
            info.id
        };

        try!(self.set_realm(realm.uri));
        send_message(&self.info, &Message::Welcome(id, WelcomeDetails::new(RouterRoles::new())))
    }

    fn handle_subscribe(&mut self, request_id: u64, options: SubscribeOptions, topic: URI) -> WampResult<()> {
        debug!("Responding to subscribe message (id: {}, topic: {})", request_id, topic.uri);
        match self.realm {
            Some(ref realm) => {
                let mut realm = realm.lock().unwrap();
                let mut manager = &mut realm.subscription_manager;
                let topic_id = {
                    let topic_id = match manager.subscriptions.subscribe_with(&topic, self.info.clone(), options.pattern_match.clone()) {
                        Ok(topic_id) => topic_id,
                        Err(e) => return Err(Error::new(ErrorKind::ErrorReason(ErrorType::Subscribe, request_id, e.reason())))
                    };
                    self.subscribed_topics.push(topic_id);
                    topic_id
                };
                manager.subscription_ids_to_uris.insert(topic_id, (topic.uri, options.pattern_match == MatchingPolicy::Prefix));
                send_message(&self.info, &Message::Subscribed(request_id, topic_id))
            },
             None => {
                Err(Error::new(ErrorKind::InvalidState("Recieved a message while not attached to a realm")))
            }
        }
    }

    fn handle_unsubscribe(&mut self, request_id: u64, topic_id: u64) -> WampResult<()> {
        match self.realm {
            Some(ref realm) => {
                let mut realm = realm.lock().unwrap();
                let mut manager = &mut realm.subscription_manager;
                let (topic_uri, is_prefix) =  match manager.subscription_ids_to_uris.get(&topic_id) {
                    Some(&(ref uri, ref is_prefix)) => (uri.clone(), is_prefix.clone()),
                    None => return Err(Error::new(ErrorKind::ErrorReason(ErrorType::Unsubscribe, request_id, Reason::NoSuchSubscription)))
                };


                let topic_id = match manager.subscriptions.unsubscribe_with(&topic_uri, &self.info, is_prefix) {
                    Ok(topic_id) => topic_id,
                    Err(e) => return Err(Error::new(ErrorKind::ErrorReason(ErrorType::Unsubscribe, request_id, e.reason())))
                };
                self.subscribed_topics.retain(|id| {
                    *id != topic_id
                });
                send_message(&self.info, &Message::Unsubscribed(request_id))
            },
            None => {
                Err(Error::new(ErrorKind::InvalidState("Recieved a message while not attached to a realm")))
            }
        }
    }

    fn handle_publish(&mut self, request_id: u64, options: PublishOptions, topic: URI, args: Option<List>, kwargs: Option<Dict>) -> WampResult<()> {
        debug!("Responding to publish message (id: {}, topic: {})", request_id, topic.uri);
        match self.realm {
            Some(ref realm) => {
                let realm = realm.lock().unwrap();
                let manager = &realm.subscription_manager;
                let publication_id = random_id();
                let mut event_message = Message::Event(1, publication_id, EventDetails::new(), args.clone(), kwargs.clone());
                let my_id = {
                    self.info.lock().unwrap().id.clone()
                };
                info!("Current topic tree: {:?}", manager.subscriptions);
                for (subscriber, topic_id, policy) in manager.subscriptions.filter(topic.clone()) {
                    if subscriber.lock().unwrap().id != my_id {
                        if let Message::Event(ref mut old_topic, ref _publish_id, ref mut details, ref _args, ref _kwargs) = event_message {
                            *old_topic = topic_id;
                            details.topic = if policy == MatchingPolicy::Strict {
                                None
                            } else {
                                Some(topic.clone())
                            };
                        }
                        try!(send_message(subscriber, &event_message));
                    }
                }
                if options.should_acknolwedge() {
                    try!(send_message(&self.info, &Message::Published(request_id, publication_id)));
                }
                Ok(())
            },
            None => {
                Err(Error::new(ErrorKind::InvalidState("Recieved a message while not attached to a realm")))
            }
        }
    }

    fn handle_goodbye(&mut self, _details: ErrorDetails, reason: Reason) -> WampResult<()> {
        let state = self.info.lock().unwrap().state.clone();
        match  state {
            ConnectionState::Initializing => {
                // TODO check specification for how this ought to work.
                Err(Error::new(ErrorKind::InvalidState("Recieved a goodbye message before handshake complete")))
            },
            ConnectionState::Connected => {
                info!("Recieved goobye message with reason: {:?}", reason);
                self.remove();
                send_message(&self.info, &Message::Goodbye(ErrorDetails::new(), Reason::GoodbyeAndOut)).ok();
                let mut info = self.info.lock().unwrap();
                info.state = ConnectionState::Disconnected;
                match info.sender.close(CloseCode::Normal) {
                    Err(e) => Err(Error::new(ErrorKind::WSError(e))),
                    _ => Ok(())
                }
            },
            ConnectionState::ShuttingDown => {
                info!("Recieved goobye message in response to our goodbye message with reason: {:?}", reason);
                let mut info = self.info.lock().unwrap();
                info.state = ConnectionState::Disconnected;
                match info.sender.close(CloseCode::Normal) {
                    Err(e) => Err(Error::new(ErrorKind::WSError(e))),
                    _ => Ok(())
                }
            },
            ConnectionState::Disconnected => {
                warn!("Recieved goodbye message after closing connection");
                Ok(())
            }
        }
    }

    fn set_realm(&mut self, realm: String) -> WampResult<()> {
        debug!("Setting realm to {}", realm);
        let realm = self.router.realms.lock().unwrap()[&realm].clone();
        {
            realm.lock().unwrap().connections.push(self.info.clone());
        }
        self.realm = Some(realm);
        Ok(())
    }

    fn process_protocol(&mut self, request: &Request, response: &mut Response) -> WSResult<()> {
        debug!("Checking protocol");
        let protocols = try!(request.protocols());
        for protocol in protocols {
            if protocol == WAMP_JSON || protocol == WAMP_MSGPACK {
                response.set_protocol(protocol);
                let mut info = self.info.lock().unwrap();
                info.protocol = protocol.to_string();
                return Ok(())
            }
        }
        Err(WSError::new(WSErrorKind::Protocol, format!("Neither {} nor {} were selected as Websocket sub-protocols", WAMP_JSON, WAMP_MSGPACK)))
    }

    fn remove(&mut self) {
        match self.realm {
            Some(ref realm) => {

                let mut realm = realm.lock().unwrap();
                {
                    let mut manager = &mut realm.subscription_manager;
                    for subscription_id in self.subscribed_topics.iter() {
                        match manager.subscription_ids_to_uris.remove(&subscription_id) {
                            Some((topic_uri, is_prefix)) => {
                                manager.subscriptions.unsubscribe_with(&topic_uri, &self.info, is_prefix).ok();
                            },
                            None => {}
                        }
                    }
                }
                let my_id = self.info.lock().unwrap().id.clone();
                realm.connections.retain(|connection| {
                    connection.lock().unwrap().id != my_id
                });
            },
            None => {
                // No need to do anything, since this connection was never added to a realm
            }
        }

    }

    fn parse_message(&self, msg: WSMessage) -> WampResult<Message> {
        match msg {
            WSMessage::Text(payload) => {
                match serde_json::from_str(&payload) {
                    Ok(message) => Ok(message),
                    Err(e) => Err(Error::new(ErrorKind::JSONError(e)))
                }
            },
            WSMessage::Binary(payload) => {
                let mut de = RMPDeserializer::new(Cursor::new(payload));
                match Deserialize::deserialize(&mut de) {
                    Ok(message) => {
                        Ok(message)
                    },
                    Err(e) => {
                        Err(Error::new(ErrorKind::MsgPackError(e)))
                    }
                }
            }
        }
    }

    fn terminate_connection(&mut self) -> WSResult<()> {
        self.remove();
        Ok(())
    }

    fn send_error(&self, err_type: ErrorType, request_id: ID, reason: Reason) -> WSResult<()> {
        send_message(&self.info, &Message::Error(err_type, request_id, HashMap::new(), reason, None, None)).map_err(|e| {
            let kind = e.get_kind();
            if let ErrorKind::WSError(e) = kind {
                e
            } else {
                WSError::new(WSErrorKind::Internal, kind.description())
            }
        })
    }

    fn on_message_error(&mut self, error: Error) -> WSResult<()> {
        use std::error::Error as StdError;
        match error.get_kind() {
            ErrorKind::WebSocketError(_) => {unimplemented!()},
            ErrorKind::WSError(e) => Err(e),
            ErrorKind::URLError(_) => {unimplemented!()},
            ErrorKind::UnexpectedMessage(msg) => {
                error!("Unexpected Message: {}", msg);
                self.terminate_connection()
            },
            ErrorKind::ThreadError(_) => {unimplemented!()},
            ErrorKind::ConnectionLost => {unimplemented!()},
            ErrorKind::JSONError(e) => {
                error!("Could not parse JSON: {}", e.description());
                self.terminate_connection()
            },
            ErrorKind::MsgPackError(e) => {
                error!("Could not parse MsgPack: {}", e.description());
                self.terminate_connection()
            },
            ErrorKind::MalformedData => {
                unimplemented!()
            },
            ErrorKind::InvalidMessageType(msg) => {
                error!("Router unable to handle message {:?}", msg);
                self.terminate_connection()
            },
            ErrorKind::InvalidState(s) => {
                error!("Invalid State: {}", s);
                self.terminate_connection()
            },
            ErrorKind::ErrorReason(err_type, id, reason) => {
                self.send_error(err_type, id, reason)
            }
        }
    }
}

impl Handler for ConnectionHandler {

    fn on_request(&mut self, request: &Request) -> WSResult<Response> {
        info!("New request");
        let mut response = match Response::from_request(request) {
            Ok(response) => response,
            Err(e) => {
                error!("Could not create response: {}", e);
                return Err(e);
            }
        };
        try!(self.process_protocol(request, &mut response));
        debug!("Sending response");
        Ok(response)
   }

    fn on_message(&mut self, msg: WSMessage) -> WSResult<()> {
        debug!("Receveied message: {:?}", msg);
        let message = match self.parse_message(msg) {
            Err(e) => return self.on_message_error(e),
            Ok(m) => m
        };
        match self.handle_message(message) {
            Err(e) => self.on_message_error(e),
            _ => Ok(())
        }
    }
}
