#![cfg_attr(feature = "serde_macros", feature(custom_derive, plugin))]
#![cfg_attr(feature = "serde_macros", plugin(serde_macros))]

extern crate serde;
extern crate serde_json;
extern crate websocket;
extern crate ws;
extern crate url;
extern crate rmp;
extern crate rmp_serde;
extern crate rand;
extern crate eventual;

#[macro_use]
extern crate log;

mod messages;
mod utils;
pub mod client;
pub mod router;

use websocket::result::WebSocketError;
use std::fmt;
use url::ParseError;
use std::sync::mpsc::SendError;
use serde_json::Error as JSONError;
use rmp_serde::decode::Error as MsgPackError;

pub use messages::{URI, Dict, List, Value};

pub type WampResult<T> = Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind
}

#[derive(Debug)]
pub enum ErrorKind {
    WebSocketError(WebSocketError),
    URLError(ParseError),
    UnexpectedMessage(&'static str),
    ThreadError(SendError<messages::Message>),
    ConnectionLost,
    JSONError(JSONError),
    MsgPackError(MsgPackError),
    MalformedData,
    UnknownTopic(String),
    InvalidState(String),
}
impl Error {
    fn new(kind: ErrorKind) -> Error {
        Error {
            kind: kind
        }
    }

    fn get_description(&self) -> String {
        format!("WAMP Error: {}", self.kind.description())
    }
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_description())
    }
}

impl ErrorKind {
    fn description(&self) -> String {
        match self {
            &ErrorKind::WebSocketError(ref e) => e.to_string(),
            &ErrorKind::UnexpectedMessage(s) => s.to_string(),
            &ErrorKind::URLError(ref e) => e.to_string(),
            &ErrorKind::ThreadError(ref e) => e.to_string(),
            &ErrorKind::ConnectionLost => "Connection Lost".to_string(),
            &ErrorKind::JSONError(ref e) => e.to_string(),
            &ErrorKind::MsgPackError(ref e) => e.to_string(),
            &ErrorKind::MalformedData => "Malformed Data".to_string(),
            &ErrorKind::UnknownTopic(ref s) => s.clone(),
            &ErrorKind::InvalidState(ref s) => s.clone(),
        }
    }
}

// #[cfg(test)]
// mod test {
//     use client::connect;
//     #[test]
//     fn test_connect() {
//         connect("ws://127.0.0.1:8090/ws", "realm1").unwrap();
//     }
// }
