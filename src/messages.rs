use std::collections::{HashMap};
use serde;
use serde::ser::Serializer;
use serde::de::Deserialize;
use serde::ser::impls::{MapIteratorVisitor, SeqIteratorVisitor};

macro_rules! try_or {
    ($e: expr, $msg: expr) => (
        match try!($e) {
            Some(val) => val,
            None => return Err(serde::de::Error::custom($msg))
        }
    );
}

type ID = u64;
type Dict = HashMap<String, Value>;
type List = Vec<Value>;

#[derive(Debug, PartialEq)]
struct URI {
    pub uri: String
}

impl URI {
    pub fn new(uri: &str) -> URI {
        URI {
            uri: uri.to_string()
        }
    }
}

//TODO deserialize this properly
#[derive(Debug, PartialEq, Deserialize)]
enum Value {
    URI(URI),
    ID(ID),
    Dict(Dict),
    Integer(u64),
    String(String),
    List(List)
}

#[derive(Hash, Eq, PartialEq, Debug)]
enum ClientRole {
    Callee,
    Caller,
    Publisher,
    Subscriber,
}

#[derive(Hash, Eq, PartialEq, Debug)]
enum RouterRole {
    Dealer,
    Broker,
}

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug)]
enum Features {
    Nope,
}

#[derive(Hash, Eq, PartialEq, Debug)]
enum Reason {
    InvalidURI,
    NoSuchProcedure,
    ProcedureAlreadyExists,
    NoSuchRegistration,
    NoSuchSubscription,
    InvalidArgument,
    SystemShutdown,
    CloseRealm,
    GoodbyeAndOut,
    NotAuthorized,
    AuthorizationFailed,
    NoSuchRealm,
    NoSuchRole,
    Cancelled,
    OptionNotAllowed,
    NoEligibleCallee,
    OptionDisallowedDiscloseMe,
    NetworkFailure
}


#[derive(Hash, Eq, PartialEq, Debug)]
enum ErrorType {
    Subscribe,
    Unsubscribe,
    Publish,
    Register,
    Unregister,
    Invocation,
    Call,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct HelloDetails {
    #[serde(default, skip_serializing_if="Option::is_none")]
    agent: Option<String>,
    roles:  HashMap<ClientRole, HashMap<String, Features>>
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct WelcomeDetails {
    #[serde(default, skip_serializing_if="Option::is_none")]
    agent: Option<String>,
    roles:  HashMap<RouterRole, HashMap<String, Features>>
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct ErrorDetails {
    #[serde(default, skip_serializing_if="Option::is_none")]
    message: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct SubscribeOptions {
    #[serde(default, rename="match", skip_serializing_if="Option::is_none")]
    pattern_match: Option<String>
}

#[derive(Debug, PartialEq)]
enum Message {
    Hello(URI, HelloDetails),
    Welcome(ID, WelcomeDetails),
    Abort(ErrorDetails, Reason),
    Goodbye(ErrorDetails, Reason),
    Error(ErrorType, ID, Dict, Reason),
    ErrorArgs(ErrorType, ID, Dict, Reason, List),
    ErrorKwArgs(ErrorType, ID, Dict, Reason, List, Dict),
    Subscribe(ID, SubscribeOptions, URI)
}

impl serde::Serialize for Message {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer,
    {
        match *self {
            Message::Hello(ref realm, ref details) => {
                (1, &realm.uri, details).serialize(serializer)
            },
            Message::Welcome(ref session, ref details) => {
                (2, session, details).serialize(serializer)
            },
            Message::Abort(ref details, ref reason) => {
                (3, details, reason).serialize(serializer)
            },
            Message::Goodbye(ref details, ref reason) => {
                (6, details, reason).serialize(serializer)
            },
            Message::Error(ref ty, id, ref details, ref reason) => {
                (8, ty, id, details, reason).serialize(serializer)
            },
            Message::ErrorArgs(ref ty, id, ref details, ref reason, ref args) => {
                (8, ty, id, details, reason, args).serialize(serializer)
            },
            Message::ErrorKwArgs(ref ty, id, ref details, ref reason, ref args, ref kwargs) => {
                (8, ty, id, details, reason, args, kwargs).serialize(serializer)
            },
            Message::Subscribe(request_id, ref options, ref topic) => {
                (32, request_id, options, topic).serialize(serializer)
            }
        }
    }
}

impl serde::Deserialize for Message {
    fn deserialize<D>(deserializer: &mut D) -> Result<Message, D::Error>
        where D: serde::Deserializer {
            deserializer.deserialize(MessageVisitor)
        }
}

struct MessageVisitor;



impl MessageVisitor {
    fn visit_hello<V>(&self,  mut visitor:V) -> Result<Message, V::Error> where V: serde::de::SeqVisitor {
        let uri = try_or!(visitor.visit(), "Hello message ended before realm uri");
        let details = try_or!(visitor.visit(), "Hello message ended before details dict");
        try!(visitor.end());
        Ok( Message::Hello(URI{uri: uri}, details))
    }

    fn visit_welcome<V>(&self,  mut visitor:V) -> Result<Message, V::Error> where V: serde::de::SeqVisitor {
        let session = try_or!(visitor.visit(), "Welcome message ended before session id");
        let details = try_or!(visitor.visit(), "Welcome message ended before details dict");
        try!(visitor.end());
        Ok( Message::Welcome(session, details))
    }

    fn visit_abort<V>(&self,  mut visitor:V) -> Result<Message, V::Error> where V: serde::de::SeqVisitor {
        let details = try_or!(visitor.visit(), "Abort message ended before details dict");
        let reason = try_or!(visitor.visit(), "Abort message ended before reason uri");
        try!(visitor.end());
        Ok( Message::Abort(details, reason))
    }

    fn visit_goodbye<V>(&self,  mut visitor:V) -> Result<Message, V::Error> where V: serde::de::SeqVisitor {
        let details = try_or!(visitor.visit(), "Goodbye message ended before details dict");
        let reason = try_or!(visitor.visit(), "Goodbye message ended before reason uri");
        try!(visitor.end());
        Ok( Message::Goodbye(details, reason))
    }

    fn visit_error<V>(&self,  mut visitor:V) -> Result<Message, V::Error> where V: serde::de::SeqVisitor {
        let message_type = try_or!(visitor.visit(), "Error message ended before message type");
        let id = try_or!(visitor.visit(), "Error message ended before session id");
        let details = try_or!(visitor.visit(), "Error message ended before details dict");
        let reason = try_or!(visitor.visit(), "Error message ended before reason uri");
        if let Some(args) = try!(visitor.visit()) {
            if let Some(kwargs) = try!(visitor.visit()) {
                try!(visitor.end());
                Ok( Message::ErrorKwArgs(message_type, id, details, reason, args, kwargs))
            } else {
                try!(visitor.end());
                Ok( Message::ErrorArgs(message_type, id, details, reason, args))
            }
        } else {
            try!(visitor.end());
            Ok( Message::Error(message_type, id, details, reason))
        }

    }

    fn visit_subscribe<V>(&self,  mut visitor:V) -> Result<Message, V::Error> where V: serde::de::SeqVisitor {
        let request = try_or!(visitor.visit(), "Subscribe message ended before request id");
        let options = try_or!(visitor.visit(), "Subscribe message ended before options dict");
        let topic = try_or!(visitor.visit(), "Subscribe message ended before topic uri");
        try!(visitor.end());
        Ok( Message::Subscribe(request, options, topic) )
    }
}

impl serde::de::Visitor for MessageVisitor {
    type Value = Message;

    fn visit_seq<V>(&mut self, mut visitor:V) -> Result<Message, V::Error> where V: serde::de::SeqVisitor {
        let message_type:u64 = try_or!(visitor.visit(), "No message type found");
        match message_type {
            1  => self.visit_hello(visitor),
            2  => self.visit_welcome(visitor),
            3  => self.visit_abort(visitor),
            6  => self.visit_goodbye(visitor),
            8  => self.visit_error(visitor),
            32 => self.visit_subscribe(visitor),
            _  => Err(serde::de::Error::custom("Unknown message type"))
        }
    }
}

impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer,
    {
        match *self {
            Value::URI(ref uri) => serializer.serialize_str(&uri.uri),
            Value::ID(ref id) => serializer.serialize_u64(*id),
            Value::Dict(ref dict) => {
                serializer.serialize_map(MapIteratorVisitor::new(
                    dict.iter(),
                    Some(dict.len()),
                ))
            },
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Integer(ref i) => serializer.serialize_u64(*i),
            Value::List(ref list) => {
                serializer.serialize_seq(SeqIteratorVisitor::new(
                    list.iter(),
                    Some(list.len()),
                ))
            }
        }
    }
}

impl serde::Serialize for Reason {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer,
    {
        let ser_str = match *self {
            Reason::InvalidURI => "wamp.error.invalid_uri",
            Reason::NoSuchProcedure => "wamp.error.no_such_procedure",
            Reason::ProcedureAlreadyExists => "wamp.error.procedure_already_exists",
            Reason::NoSuchRegistration => "wamp.error.no_such_registration",
            Reason::NoSuchSubscription => "wamp.error.no_such_subscription",
            Reason::InvalidArgument => "wamp.error.invalid_argument",
            Reason::SystemShutdown => "wamp.error.system_shutdown",
            Reason::CloseRealm => "wamp.error.close_realm",
            Reason::GoodbyeAndOut => "wamp.error.goodbye_and_out",
            Reason::NotAuthorized => "wamp.error.not_authorized",
            Reason::AuthorizationFailed => "wamp.error.authorization_failed",
            Reason::NoSuchRealm => "wamp.error.no_such_realm",
            Reason::NoSuchRole => "wamp.error.no_such_role",
            Reason::Cancelled => "wamp.error.cancelled",
            Reason::OptionNotAllowed => "wamp.error.option_not_allowed",
            Reason::NoEligibleCallee => "wamp.error.no_eligible_callee",
            Reason::OptionDisallowedDiscloseMe => "wamp.error.option-disallowed.disclose_me",
            Reason::NetworkFailure => "wamp.error.network_failure"
        };
        serializer.serialize_str(ser_str)
    }
}

impl serde::Deserialize for Reason {
    fn deserialize<D>(deserializer: &mut D) -> Result<Reason, D::Error>
        where D: serde::Deserializer,
    {
        deserializer.deserialize(ReasonVisitor)
    }
}

struct ReasonVisitor;

impl serde::de::Visitor for ReasonVisitor {
    type Value = Reason;

    fn visit_str<E>(&mut self, value: &str) -> Result<Reason, E>
        where E: serde::de::Error,
    {
        match value {
             "wamp.error.invalid_uri" => Ok(Reason::InvalidURI),
             "wamp.error.no_such_procedure" => Ok(Reason::NoSuchProcedure),
             "wamp.error.procedure_already_exists" => Ok(Reason::ProcedureAlreadyExists),
             "wamp.error.no_such_registration" => Ok(Reason::NoSuchRegistration),
             "wamp.error.no_such_subscription" => Ok(Reason::NoSuchSubscription),
             "wamp.error.invalid_argument" => Ok(Reason::InvalidArgument),
             "wamp.error.system_shutdown" => Ok(Reason::SystemShutdown),
             "wamp.error.close_realm" => Ok(Reason::CloseRealm),
             "wamp.error.goodbye_and_out" => Ok(Reason::GoodbyeAndOut),
             "wamp.error.not_authorized" => Ok(Reason::NotAuthorized),
             "wamp.error.authorization_failed" => Ok(Reason::AuthorizationFailed),
             "wamp.error.no_such_realm" => Ok(Reason::NoSuchRealm),
             "wamp.error.no_such_role" => Ok(Reason::NoSuchRole),
             "wamp.error.cancelled" => Ok(Reason::Cancelled),
             "wamp.error.option_not_allowed" => Ok(Reason::OptionNotAllowed),
             "wamp.error.no_eligible_callee" => Ok(Reason::NoEligibleCallee),
             "wamp.error.option-disallowed.disclose_me" => Ok(Reason::OptionDisallowedDiscloseMe),
             "wamp.error.network_failure" => Ok(Reason::NetworkFailure),
            x => Err(serde::de::Error::custom(format!("Invalid error uri {}", x)))
        }
    }

}

impl serde::Serialize for ClientRole {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer,
    {
        let ser_str = match *self {
            ClientRole::Callee => "callee",
            ClientRole::Caller => "caller",
            ClientRole::Publisher => "publisher",
            ClientRole::Subscriber => "subscriber",
        };
        serializer.serialize_str(ser_str)
    }
}

impl serde::Deserialize for ClientRole {
    fn deserialize<D>(deserializer: &mut D) -> Result<ClientRole, D::Error>
        where D: serde::Deserializer,
    {
        deserializer.deserialize(ClientRoleVisitor)
    }
}

struct ClientRoleVisitor;

impl serde::de::Visitor for ClientRoleVisitor {
    type Value = ClientRole;

    fn visit_str<E>(&mut self, value: &str) -> Result<ClientRole, E>
        where E: serde::de::Error,
    {
        match value {
            "callee" => Ok(ClientRole::Callee),
            "caller" => Ok(ClientRole::Caller),
            "publisher" => Ok(ClientRole::Publisher),
            "subscriber" => Ok(ClientRole::Subscriber),
            x => Err(serde::de::Error::custom(format!("Invalid role for client: {}", x)))
        }
    }

}

impl serde::Serialize for ErrorType {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer,
    {
        let ser_int = match *self {
             ErrorType::Subscribe => 32,
             ErrorType::Unsubscribe => 34,
             ErrorType::Publish => 16,
             ErrorType::Register => 64,
             ErrorType::Unregister => 66,
             ErrorType::Invocation => 68,
             ErrorType::Call => 48,
        };
        serializer.serialize_u64(ser_int)
    }
}

impl serde::Deserialize for ErrorType {
    fn deserialize<D>(deserializer: &mut D) -> Result<ErrorType, D::Error>
        where D: serde::Deserializer,
    {
        deserializer.deserialize(ErrorTypeVisitor)
    }
}



struct ErrorTypeVisitor;

impl serde::de::Visitor for ErrorTypeVisitor {
    type Value = ErrorType;

    fn visit_u64<E>(&mut self, value: u64) -> Result<ErrorType, E>
        where E: serde::de::Error,
    {
        match value {
            32 => Ok(ErrorType::Subscribe),
            34 => Ok(ErrorType::Unsubscribe),
            16 => Ok(ErrorType::Publish),
            64 => Ok(ErrorType::Register),
            66 => Ok(ErrorType::Unregister),
            68 => Ok(ErrorType::Invocation),
            48 => Ok(ErrorType::Call),
            x => Err(serde::de::Error::custom(format!("Invalid message error type: {}", x)))
        }
    }

}

impl serde::Serialize for URI {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer,
    {
        serializer.serialize_str(&self.uri)
    }
}

impl serde::Deserialize for URI {
    fn deserialize<D>(deserializer: &mut D) -> Result<URI, D::Error>
        where D: serde::Deserializer,
    {
        deserializer.deserialize(URIVisitor)
    }
}



struct URIVisitor;

impl serde::de::Visitor for URIVisitor {
    type Value = URI;

    fn visit_str<E>(&mut self, value: &str) -> Result<URI, E>
        where E: serde::de::Error,
    {
        Ok(URI {
            uri: value.to_string()
        })
    }

}


impl serde::Serialize for RouterRole {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer,
    {
        let ser_str = match *self {
            RouterRole::Dealer => "dealer",
            RouterRole::Broker => "broker",
        };
        serializer.serialize_str(ser_str)
    }
}

impl serde::Deserialize for RouterRole {
    fn deserialize<D>(deserializer: &mut D) -> Result<RouterRole, D::Error>
        where D: serde::Deserializer,
    {
        deserializer.deserialize(RouterRoleVisitor)
    }
}

struct RouterRoleVisitor;

impl serde::de::Visitor for RouterRoleVisitor {
    type Value = RouterRole;

    fn visit_str<E>(&mut self, value: &str) -> Result<RouterRole, E>
        where E: serde::de::Error,
    {
        match value {
            "dealer" => Ok(RouterRole::Dealer),
            "broker" => Ok(RouterRole::Broker),
            x => Err(serde::de::Error::custom(format!("Invalid router role: {}", x)))
        }
    }

}

#[cfg(test)]
mod test {
    use super::{Message, URI, ClientRole, RouterRole, HelloDetails, WelcomeDetails, ErrorDetails, Reason, ErrorType, SubscribeOptions};
    use std::collections::{HashMap};
    use serde_json;

    macro_rules! two_way_test {
        ($message: expr, $s: expr) => (
            {
            let message = $message;
            assert_eq!(serde_json::to_string(&message).unwrap(), $s);
            assert_eq!(serde_json::from_str::<Message>($s).unwrap(), message);
        }
        );
    }

    #[test]
    fn serialize_hello() {
        two_way_test!(
            Message::Hello(URI{uri: "ca.dal.wamp.test".to_string()}, HelloDetails{agent: None, roles: HashMap::new()}),
            "[1,\"ca.dal.wamp.test\",{\"roles\":{}}]"
        );
        let mut roles_map = HashMap::new();
        roles_map.insert(ClientRole::Caller, HashMap::new());
        two_way_test!(
            Message::Hello(URI{uri: "ca.dal.wamp.test".to_string()}, HelloDetails{agent: Some("dal_wamp".to_string()), roles: roles_map}),
            "[1,\"ca.dal.wamp.test\",{\"agent\":\"dal_wamp\",\"roles\":{\"caller\":{}}}]"
        )
    }

    #[test]
    fn serialize_welcome() {
        two_way_test!(
            Message::Welcome(493782, WelcomeDetails{agent: None, roles: HashMap::new()}),
            "[2,493782,{\"roles\":{}}]"
        );
        let mut roles_map = HashMap::new();
        roles_map.insert(RouterRole::Broker, HashMap::new());
        two_way_test!(
            Message::Welcome(493782, WelcomeDetails{agent: Some("dal_wamp".to_string()), roles: roles_map}),
            "[2,493782,{\"agent\":\"dal_wamp\",\"roles\":{\"broker\":{}}}]"
        );
    }


    #[test]
    fn serialize_abort() {
        two_way_test!(
            Message::Abort(ErrorDetails{message: None,}, Reason::NoSuchRealm),
            "[3,{},\"wamp.error.no_such_realm\"]"
        );
        two_way_test!(
            Message::Abort(ErrorDetails{message: Some("The realm does not exist".to_string())}, Reason::NoSuchRealm),
            "[3,{\"message\":\"The realm does not exist\"},\"wamp.error.no_such_realm\"]"
        );
    }

    #[test]
    fn serialize_goodbye() {
        two_way_test!(
            Message::Goodbye(ErrorDetails{message: None,}, Reason::GoodbyeAndOut),
            "[6,{},\"wamp.error.goodbye_and_out\"]"
        );
        two_way_test!(
            Message::Goodbye(ErrorDetails{message: Some("The host is shutting down now".to_string())}, Reason::SystemShutdown),
            "[6,{\"message\":\"The host is shutting down now\"},\"wamp.error.system_shutdown\"]"
        );
    }


    #[test]
    fn serialize_error() {
        two_way_test!(
            Message::Error(ErrorType::Subscribe, 713845233, HashMap::new(), Reason::NotAuthorized),
            "[8,32,713845233,{},\"wamp.error.not_authorized\"]"
        );

        two_way_test!(
            Message::ErrorArgs(ErrorType::Unsubscribe, 3746383, HashMap::new(), Reason::InvalidURI, Vec::new()),
            "[8,34,3746383,{},\"wamp.error.invalid_uri\",[]]"
        );

        two_way_test!(
            Message::ErrorKwArgs(ErrorType::Register, 8534533, HashMap::new(), Reason::InvalidArgument, Vec::new(), HashMap::new()),
            "[8,64,8534533,{},\"wamp.error.invalid_argument\",[],{}]"
        );
    }

    #[test]
    fn serialize_subscribe() {
        two_way_test!(
            Message::Subscribe(58944, SubscribeOptions{pattern_match: None}, URI::new("ca.dal.test.the_sub")),
            "[32,58944,{},\"ca.dal.test.the_sub\"]"
        )
    }




}
