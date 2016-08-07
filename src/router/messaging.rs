use super::{ConnectionHandler, ConnectionInfo, WAMP_JSON};
use ws::{Sender, Handler, Message as WSMessage, Error as WSError, ErrorKind as WSErrorKind, Result as WSResult, Request, Response,};
use std::sync::{Arc, Mutex};

use std::collections::{HashMap};
use serde_json;
use serde::{Deserialize, Serialize};
use rmp_serde::Deserializer as RMPDeserializer;
use rmp_serde::Serializer;
use utils::StructMapWriter;
use std::io::Cursor;
use messages::{Message, ErrorType, Reason};
use ::{ ID, WampResult, Error, ErrorKind};


pub fn send_message(info: &Arc<Mutex<ConnectionInfo>>, message: &Message) -> WampResult<()> {
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

impl ConnectionHandler {

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
            },
            Message::Register(request_id, options, procedure) => {
                self.handle_register(request_id, options, procedure)
            },
            Message::Unregister(request_id, procedure_id) => {
                self.handle_unregister(request_id, procedure_id)
            },
            t => {
                Err(Error::new(ErrorKind::InvalidMessageType(t)))
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
