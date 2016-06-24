pub mod patterns;

use ws::{listen as ws_listen, Sender, Handler, Message as WSMessage, Result, Error, ErrorKind, Request, Response, CloseCode};
use std::sync::{Arc, Mutex, RwLock};
use std::cell::RefCell;
use std::collections::{HashMap};
use serde_json;
use serde::{Deserialize, Serialize};
use rmp_serde::Deserializer as RMPDeserializer;
use rmp_serde::Serializer;
use utils::StructMapWriter;
use std::io::Cursor;
use messages::{Message, URI, HelloDetails, WelcomeDetails, RouterRoles, SubscribeOptions, PublishOptions, EventDetails, ErrorDetails, Reason};
use ::{List, Dict, ID, MatchingPolicy};
use std::marker::PhantomData;
use rand::{thread_rng};
use rand::distributions::{Range, IndependentSample};
use router::patterns::PatternNode;

struct SubscriptionManager {
    subscriptions : PatternNode<Arc<RefCell<ConnectionInfo>>>,
    subscription_ids_to_uris: HashMap<u64, (String, bool)>, // Should only be accessed when the subscriptions mutex is open
}

struct Realm {
    subscription_manager: Mutex<SubscriptionManager>,
    connections: Vec<Arc<RefCell<ConnectionInfo>>>
}

pub struct Router {
    info: Arc<RefCell<RouterInfo>>
}

struct RouterInfo {
    realms: HashMap<String, Arc<RefCell<Realm>>>,
    realm_marker: RwLock<PhantomData<u8>>

}

struct ConnectionHandler {
    info: Arc<RefCell<ConnectionInfo>>,
    router: Arc<RefCell<RouterInfo>>,
    realm: Option<Arc<RefCell<Realm>>>,
}

pub struct ConnectionInfo {
    state: ConnectionState,
    sender: Sender,
    protocol: String,
    id: u64,
    subscribed_topics: Vec<ID>
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

fn send_message(info: &Arc<RefCell<ConnectionInfo>>, message: &Message) -> Result<()> {
    let info = info.borrow();

    debug!("Sending message {:?} via {}", message, info.protocol);
    if info.protocol == WAMP_JSON {
        send_message_json(&info.sender, message)
    } else {
        send_message_msgpack(&info.sender, message)
    }
}

fn send_message_json(sender: &Sender, message: &Message) -> Result<()> {
    // Send the message
    sender.send(WSMessage::Text(serde_json::to_string(message).unwrap()))

}

fn send_message_msgpack(sender: &Sender, message: &Message) -> Result<()> {

    // Send the message
    let mut buf: Vec<u8> = Vec::new();
    message.serialize(&mut Serializer::with(&mut buf, StructMapWriter)).unwrap();
    sender.send(WSMessage::Binary(buf))

}

impl Router {
    #[inline]
    pub fn new() -> Router {
        Router{
            info: Arc::new(RefCell::new(RouterInfo {
                realms: HashMap::new(),
                realm_marker: RwLock::new(PhantomData)
            }))
        }
    }

    pub fn listen(self, url: &str) {

        ws_listen(url, |sender| {
            ConnectionHandler {
                info: Arc::new(RefCell::new(ConnectionInfo{
                    state: ConnectionState::Initializing,
                    sender: sender,
                    protocol: String::new(),
                    id: random_id(),
                    subscribed_topics: Vec::new(),
                })),
                realm: None,
                router: self.info.clone(),
            }
        }).unwrap()
    }

    pub fn add_realm(&mut self, realm: &str) {
        let mut info = self.info.borrow_mut();
        if info.realms.contains_key(realm) {
            return
        }
        let _ = info.realm_marker.write().unwrap();
        info.realms.insert(realm.to_string(), Arc::new(RefCell::new(Realm {
            connections: Vec::new(),
            subscription_manager: Mutex::new(SubscriptionManager {
                subscriptions: PatternNode::new(),
                subscription_ids_to_uris: HashMap::new()
            })
        })));
        debug!("Added realm {}", realm);
    }
}



impl ConnectionHandler{
    fn handle_message(&mut self, message: Message) -> Result<()> {
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
            _ => {
                Err(Error::new(ErrorKind::Internal, format!("Invalid message type: {:?}", message)))
            }
        }
    }

    fn handle_hello(&mut self, realm: URI, _details: HelloDetails) -> Result<()> {
        debug!("Responding to hello message (realm: {:?})", realm);
        {
            self.info.borrow_mut().state = ConnectionState::Connected;
        }
        try!(self.set_realm(realm.uri));
        let id = {self.info.borrow().id};
        send_message(&self.info, &Message::Welcome(id, WelcomeDetails::new(RouterRoles::new())))
    }

    fn handle_subscribe(&mut self, request_id: u64, options: SubscribeOptions, topic: URI) -> Result<()> {
        debug!("Responding to subscribe message (id: {}, topic: {})", request_id, topic.uri);
        match self.realm {
            Some(ref realm) => {
                let realm = realm.borrow();
                let mut manager = realm.subscription_manager.lock().unwrap();
                let topic_id = {
                    // TODO fix the error issues so that we can just try! this
                    let topic_id = manager.subscriptions.subscribe_with(&topic, self.info.clone(), options.pattern_match.clone()).unwrap();
                    self.info.borrow_mut().subscribed_topics.push(topic_id);
                    topic_id
                };
                manager.subscription_ids_to_uris.insert(topic_id, (topic.uri, options.pattern_match == MatchingPolicy::Prefix));
                send_message(&self.info, &Message::Subscribed(request_id, topic_id))
            },
             None => {
                // TODO But actually handle the eror here
                Ok(())
            }
        }
    }

    fn handle_unsubscribe(&mut self, request_id: u64, topic_id: u64) -> Result<()> {
        match self.realm {
            Some(ref realm) => {
                let realm = realm.borrow();
                let mut manager = realm.subscription_manager.lock().unwrap();
                let (topic_uri, is_prefix) = {
                        match manager.subscription_ids_to_uris.get(&topic_id) {
                        Some(&(ref uri, ref is_prefix)) => (uri.clone(), is_prefix.clone()),
                        None      => return Err(Error::new(ErrorKind::Internal, "No topic with that id"))
                    }
                };

                // TODO Also fix this error situation
                let topic_id = manager.subscriptions.unsubscribe_with(&topic_uri, &self.info, is_prefix).unwrap();
                self.info.borrow_mut().subscribed_topics.retain(|id| {
                    *id != topic_id
                });
                send_message(&self.info, &Message::Unsubscribed(request_id))
            },
            None => {
                // TODO But actually handle the error here
                Ok(())
            }
        }
    }

    fn handle_publish(&mut self, request_id: u64, options: PublishOptions, topic: URI, args: Option<List>, kwargs: Option<Dict>) -> Result<()> {
        debug!("Responding to publish message (id: {}, topic: {})", request_id, topic.uri);
        match self.realm {
            Some(ref realm) => {
                let realm = realm.borrow();
                let manager = realm.subscription_manager.lock().unwrap();
                let publication_id = random_id();
                let mut event_message = Message::Event(1, publication_id, EventDetails::new(), args.clone(), kwargs.clone());
                let my_id = self.info.borrow().id;
                for (subscriber, topic_id, policy) in manager.subscriptions.filter(topic.clone()) {
                    if subscriber.borrow().id != my_id {
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
                // TODO But actually handle the eror here
                Ok(())
            }
        }
    }

    fn handle_goodbye(&mut self, _details: ErrorDetails, reason: Reason) -> Result<()> {
        let state = self.info.borrow().state.clone();
        match  state {
            ConnectionState::Initializing => {
                // TODO check specification for how this ought to work.
                Err(Error::new(ErrorKind::Internal, "Recieved a goodbye message before handshake complete"))
            },
            ConnectionState::Connected => {
                info!("Recieved goobye message with reason: {:?}", reason);
                self.remove();
                send_message(&self.info, &Message::Goodbye(ErrorDetails::new(), Reason::GoodbyeAndOut)).ok();
                let mut info = self.info.borrow_mut();
                info.state = ConnectionState::Disconnected;
                info.sender.close(CloseCode::Normal)
            },
            ConnectionState::ShuttingDown => {
                info!("Recieved goobye message in response to our goodbye message with reason: {:?}", reason);
                let mut info = self.info.borrow_mut();
                info.state = ConnectionState::Disconnected;
                info.sender.close(CloseCode::Normal)
            },
            ConnectionState::Disconnected => {
                warn!("Recieved goodbye message after closing connection");
                Ok(())
            }
        }
    }

    fn set_realm(&mut self, realm: String) -> Result<()> {
        debug!("Setting realm to {}", realm);
        let router = self.router.borrow_mut();
        let _ = router.realm_marker.read().unwrap();
        let realm = router.realms[&realm].clone();
        {
            realm.borrow_mut().connections.push(self.info.clone());
        }
        self.realm = Some(realm);
        Ok(())
    }

    fn process_protocol(&mut self, request: &Request, response: &mut Response) -> Result<()> {
        debug!("Checking protocol");
        let protocols = try!(request.protocols());
        for protocol in protocols {
            if protocol == WAMP_JSON || protocol == WAMP_MSGPACK {
                response.set_protocol(protocol);
                let mut info = self.info.borrow_mut();
                info.protocol = protocol.to_string();
                return Ok(())
            }
        }
        Err(Error::new(ErrorKind::Protocol, format!("Neither {} nor {} were selected as Websocket sub-protocols", WAMP_JSON, WAMP_MSGPACK)))
    }

    fn remove(&mut self) {
        match self.realm {
            Some(ref realm) => {
                {
                    let realm = realm.borrow();
                    let mut manager = realm.subscription_manager.lock().unwrap();
                    for subscription_id in self.info.borrow().subscribed_topics.iter() {
                        // TODO Error check
                        let (topic_uri, is_prefix) = manager.subscription_ids_to_uris.remove(&subscription_id).unwrap();
                        // TODO More error checking
                        manager.subscriptions.unsubscribe_with(&topic_uri, &self.info, is_prefix).unwrap();
                    }
                }
                realm.borrow_mut().connections.retain(|connection| {
                    connection.borrow().id != self.info.borrow().id
                });
            },
            None => {
                // No need to do anything, since this connection was never added to a realm
            }
        }

    }
}

impl Handler for ConnectionHandler {

    fn on_request(&mut self, request: &Request) -> Result<Response> {
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

    fn on_message(&mut self, msg: WSMessage) -> Result<()> {
        debug!("Receveied message: {:?}", msg);
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
                        error!("WHAT KIND OF MESSAGE DO YOU THINK THIS IS?");
                        Err(Error::new(ErrorKind::Custom(Box::new(e)), "unable to parse message"))
                    }
                }
            }
        });

        self.handle_message(message)
    }
}
