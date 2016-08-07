use super::{ConnectionHandler, random_id};

use router::messaging::send_message;
use messages::{Message, URI, RegisterOptions, PublishOptions, EventDetails, ErrorType, Reason};
use ::{List, Dict,  MatchingPolicy, WampResult, Error, ErrorKind};

impl ConnectionHandler{
    pub fn handle_register(&mut self, request_id: u64, options: RegisterOptions, procedure: URI) -> WampResult<()> {
        debug!("Responding to register message (id: {}, procedure: {})", request_id, procedure.uri);
        match self.realm {
            Some(ref realm) => {
                let mut realm = realm.lock().unwrap();
                let mut manager = &mut realm.registration_manager;
                let procedure_id = {
                    let procedure_id = match manager.registrations.subscribe_with(&procedure, self.info.clone(), options.pattern_match.clone()) {
                        Ok(procedure_id) => procedure_id,
                        Err(e) => return Err(Error::new(ErrorKind::ErrorReason(ErrorType::Register, request_id, e.reason())))
                    };
                    self.registered_procedures.push(procedure_id);
                    procedure_id
                };
                manager.registration_ids_to_uris.insert(procedure_id, (procedure.uri, options.pattern_match == MatchingPolicy::Prefix));
                send_message(&self.info, &Message::Registered(request_id, procedure_id))
            },
             None => {
                Err(Error::new(ErrorKind::InvalidState("Recieved a message while not attached to a realm")))
            }
        }
    }

    pub fn handle_unregister(&mut self, request_id: u64, procedure_id: u64) -> WampResult<()> {
        match self.realm {
            Some(ref realm) => {
                let mut realm = realm.lock().unwrap();
                let mut manager = &mut realm.registration_manager;
                let (procedure_uri, is_prefix) =  match manager.registration_ids_to_uris.get(&procedure_id) {
                    Some(&(ref uri, ref is_prefix)) => (uri.clone(), is_prefix.clone()),
                    None => return Err(Error::new(ErrorKind::ErrorReason(ErrorType::Unregister, request_id, Reason::NoSuchSubscription)))
                };


                let procedure_id = match manager.registrations.unsubscribe_with(&procedure_uri, &self.info, is_prefix) {
                    Ok(procedure_id) => procedure_id,
                    Err(e) => return Err(Error::new(ErrorKind::ErrorReason(ErrorType::Unregister, request_id, e.reason())))
                };
                self.registered_procedures.retain(|id| {
                    *id != procedure_id
                });
                send_message(&self.info, &Message::Unregistered(request_id))
            },
            None => {
                Err(Error::new(ErrorKind::InvalidState("Recieved a message while not attached to a realm")))
            }
        }
    }

    // pub fn handle_publish(&mut self, request_id: u64, options: PublishOptions, procedure: URI, args: Option<List>, kwargs: Option<Dict>) -> WampResult<()> {
    //     debug!("Responding to publish message (id: {}, procedure: {})", request_id, procedure.uri);
    //     match self.realm {
    //         Some(ref realm) => {
    //             let realm = realm.lock().unwrap();
    //             let manager = &realm.registration_manager;
    //             let publication_id = random_id();
    //             let mut event_message = Message::Event(1, publication_id, EventDetails::new(), args.clone(), kwargs.clone());
    //             let my_id = {
    //                 self.info.lock().unwrap().id.clone()
    //             };
    //             info!("Current procedure tree: {:?}", manager.registrations);
    //             for (registrant, procedure_id, policy) in manager.registrations.filter(procedure.clone()) {
    //                 if registrant.lock().unwrap().id != my_id {
    //                     if let Message::Event(ref mut old_procedure, ref _publish_id, ref mut details, ref _args, ref _kwargs) = event_message {
    //                         *old_procedure = procedure_id;
    //                         details.procedure = if policy == MatchingPolicy::Strict {
    //                             None
    //                         } else {
    //                             Some(procedure.clone())
    //                         };
    //                     }
    //                     try!(send_message(registrant, &event_message));
    //                 }
    //             }
    //             if options.should_acknowledge() {
    //                 try!(send_message(&self.info, &Message::Published(request_id, publication_id)));
    //             }
    //             Ok(())
    //         },
    //         None => {
    //             Err(Error::new(ErrorKind::InvalidState("Recieved a message while not attached to a realm")))
    //         }
    //     }
    // }

}
