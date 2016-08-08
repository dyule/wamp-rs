mod handshake;
mod messaging;
mod pubsub;
mod rpc;

use ws::{listen as ws_listen, Sender, Result as WSResult };
use std::sync::{Arc, Mutex};
use std::collections::{HashMap};
use std::marker::Sync;
use rand::{thread_rng};
use rand::distributions::{Range, IndependentSample};
use router::pubsub::SubscriptionPatternNode;
use router::rpc::RegistrationPatternNode;
use super::ID;


struct SubscriptionManager {
    subscriptions : SubscriptionPatternNode<Arc<Mutex<ConnectionInfo>>>,
    subscription_ids_to_uris: HashMap<u64, (String, bool)>
}

struct RegistrationManager {
    registrations : RegistrationPatternNode<Arc<Mutex<ConnectionInfo>>>,
    registration_ids_to_uris: HashMap<u64, (String, bool)>,
    active_calls: HashMap<ID, (ID, Arc<Mutex<ConnectionInfo>>)>
}

struct Realm {
    subscription_manager: SubscriptionManager,
    registration_manager: RegistrationManager,
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
    subscribed_topics: Vec<ID>,
    registered_procedures: Vec<ID>,
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
    // TODO make this a constant
    let between = Range::new(0, 1u64.rotate_left(56) - 1);
    between.ind_sample(&mut rng)
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
                registered_procedures: Vec::new(),
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
                subscriptions: SubscriptionPatternNode::new(),
                subscription_ids_to_uris: HashMap::new()
            },
            registration_manager: RegistrationManager {
                registrations: RegistrationPatternNode::new(),
                registration_ids_to_uris: HashMap::new(),
                active_calls: HashMap::new()
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
                {
                    let mut manager = &mut realm.registration_manager;
                    for registration_id in self.registered_procedures.iter() {
                        match manager.registration_ids_to_uris.remove(&registration_id) {
                            Some((topic_uri, is_prefix)) => {
                                manager.registrations.unregister_with(&topic_uri, &self.info, is_prefix).ok();
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


    fn terminate_connection(&mut self) -> WSResult<()> {
        self.remove();
        Ok(())
    }

}
