#![cfg_attr(feature = "cargo-clippy", allow(match_same_arms))]
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate eventual;
extern crate rand;
extern crate rmp;
extern crate rmp_serde;
extern crate url;
extern crate ws;

#[macro_use]
extern crate log;

pub mod client;
mod messages;
pub mod router;
mod utils;

use rmp_serde::decode::Error as MsgPackError;
use serde_json::Error as JSONError;
use std::fmt;
use std::sync::mpsc::SendError;
use url::ParseError;
use ws::Error as WSError;

pub use client::{Client, Connection};
pub use messages::{ArgDict, ArgList, CallError, Dict, InvocationPolicy, List, MatchingPolicy,
                   Reason, Value, URI};
use messages::{ErrorType, Message};
pub use router::Router;

pub type CallResult<T> = Result<T, CallError>;
pub type WampResult<T> = Result<T, Error>;
pub type ID = u64;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    WSError(WSError),
    URLError(ParseError),
    HandshakeError(Reason),
    UnexpectedMessage(&'static str), // Used when a peer receives another message before Welcome or Hello
    ThreadError(SendError<messages::Message>),
    ConnectionLost,
    Closing(String),
    JSONError(JSONError),
    MsgPackError(MsgPackError),
    MalformedData,
    InvalidMessageType(Message),
    InvalidState(&'static str),
    Timeout,
    ErrorReason(ErrorType, ID, Reason),
}
impl Error {
    fn new(kind: ErrorKind) -> Error {
        Error { kind: kind }
    }

    fn get_description(&self) -> String {
        format!("WAMP Error: {}", self.kind.description())
    }

    #[inline]
    fn get_kind(self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_description())
    }
}

impl ErrorKind {
    fn description(&self) -> String {
        match *self {
            ErrorKind::WSError(ref e) => e.to_string(),
            ErrorKind::URLError(ref e) => e.to_string(),
            ErrorKind::HandshakeError(ref r) => r.to_string(),
            ErrorKind::ThreadError(ref e) => e.to_string(),
            ErrorKind::JSONError(ref e) => e.to_string(),
            ErrorKind::MsgPackError(ref e) => e.to_string(),
            ErrorKind::ErrorReason(_, _, ref s) => s.to_string(),
            ErrorKind::Closing(ref s) => s.clone(),
            ErrorKind::UnexpectedMessage(s) | ErrorKind::InvalidState(s) => s.to_string(),
            ErrorKind::ConnectionLost => "Connection Lost".to_string(),
            ErrorKind::MalformedData => "Malformed Data".to_string(),
            ErrorKind::Timeout => "Connection timed out".to_string(),
            ErrorKind::InvalidMessageType(ref t) => format!("Invalid Message Type: {:?}", t),
        }
    }
}
