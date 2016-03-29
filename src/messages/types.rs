use std::collections::{HashMap};
use serde;
use serde::ser::Serializer;
use serde::de::{
    Deserialize,
    MapVisitor,
    SeqVisitor
};

fn invert(b: &bool) -> bool {
    !*b
}

/**************************
         Types
**************************/

pub type ID = u64;
pub type Dict = HashMap<String, Value>;
pub type List = Vec<Value>;

/**************************
         Structs
**************************/

// TODO properly implement Hash and Eq
#[derive(Debug, PartialEq, Clone)]
pub struct URI {
    pub uri: String
}

impl URI {
    pub fn new(uri: &str) -> URI {
        URI {
            uri: uri.to_string()
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Value {
    // The ID and URI types cannot be distinguished from string and integer types respectively.
    // So, we just ignore them here
    Dict(Dict),
    Integer(u64),
    String(String),
    List(List),
    Boolean(bool)
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum ClientRole {
    Callee,
    Caller,
    Publisher,
    Subscriber,
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum RouterRole {
    Dealer,
    Broker,
}

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug)]
pub enum Features {
    Nope,
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum Reason {
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
pub enum ErrorType {
    Subscribe,
    Unsubscribe,
    Publish,
    Register,
    Unregister,
    Invocation,
    Call,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct HelloDetails {
    #[serde(default, skip_serializing_if="Option::is_none")]
    agent: Option<String>,
    roles:  HashMap<ClientRole, HashMap<String, Value>>
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct WelcomeDetails {
    #[serde(default, skip_serializing_if="Option::is_none")]
    agent: Option<String>,
    roles:  HashMap<RouterRole, HashMap<String, Value>>
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ErrorDetails {
    #[serde(default, skip_serializing_if="Option::is_none")]
    message: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct SubscribeOptions {
    #[serde(default, rename="match", skip_serializing_if="Option::is_none")]
    pattern_match: Option<String>
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct PublishOptions {
    #[serde(default, skip_serializing_if="invert")]
    acknolwedge: bool
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct EventDetails {
    #[serde(default, skip_serializing_if="Option::is_none")]
    publisher: Option<String>,

    #[serde(default, skip_serializing_if="Option::is_none")]
    trustlevel: Option<u64>,
}

/**************************
        Visitors
**************************/

struct RouterRoleVisitor;
struct URIVisitor;
struct ErrorTypeVisitor;
struct ClientRoleVisitor;
struct ReasonVisitor;
struct ValueVisitor;


/**************************
      Implementations
**************************/
impl HelloDetails {
    pub fn new(roles: HashMap<ClientRole, HashMap<String, Value>>) -> HelloDetails {
        HelloDetails {
            roles: roles,
            agent: None
        }
    }

    pub fn new_with_agent(roles: HashMap<ClientRole, HashMap<String, Value>>, agent: &str) -> HelloDetails {
        HelloDetails {
            roles: roles,
            agent: Some(agent.to_string())
        }
    }

}

impl WelcomeDetails {
    pub fn new(roles: HashMap<RouterRole, HashMap<String, Value>>) -> WelcomeDetails {
        WelcomeDetails {
            roles: roles,
            agent: None
        }
    }

    pub fn new_with_agent(roles: HashMap<RouterRole, HashMap<String, Value>>, agent: &str) -> WelcomeDetails {
        WelcomeDetails {
            roles: roles,
            agent: Some(agent.to_string())
        }
    }

}

impl ErrorDetails {
    pub fn new() -> ErrorDetails {
        ErrorDetails {
            message: None
        }
    }


    pub fn new_with_message(message: &str) -> ErrorDetails {
        ErrorDetails {
            message: Some(message.to_string())
        }
    }
}

impl SubscribeOptions {
    pub fn new() -> SubscribeOptions {
        SubscribeOptions {
            pattern_match: None
        }
    }
}

impl PublishOptions {
    pub fn new(acknolwedge: bool) -> PublishOptions {
        PublishOptions {
            acknolwedge: acknolwedge
        }
    }
}

impl EventDetails {
    pub fn new() -> EventDetails {
        EventDetails {
            publisher: None,
            trustlevel: None
        }
    }
}


/**************************
 Serializers/Deserializers
**************************/

/*-------------------------
         Value
-------------------------*/
impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer,
    {
        match *self {
            Value::Dict(ref dict) => dict.serialize(serializer),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Integer(i) => serializer.serialize_u64(i),
            Value::List(ref list) => list.serialize(serializer),
            Value::Boolean(b) => serializer.serialize_bool(b)
        }
    }
}

impl serde::Deserialize for Value {
    fn deserialize<D>(deserializer: &mut D) -> Result<Value, D::Error>
        where D: serde::Deserializer,
    {
        deserializer.deserialize(ValueVisitor)
    }
}


// XXX Right now there is no way to tell the difference between a URI and a string, or an ID and an Integer
impl serde::de::Visitor for ValueVisitor {
    type Value = Value;


    #[inline]
    fn visit_str<E>(&mut self, value: &str) -> Result<Value, E>
        where E: serde::de::Error {
            Ok(Value::String(value.to_string()))
    }


    #[inline]
    fn visit_u64<E>(&mut self, value: u64) -> Result<Value, E>
    where E: serde::de::Error {
        Ok(Value::Integer(value))
    }

    #[inline]
    fn visit_bool<E>(&mut self, value: bool) -> Result<Value, E>
    where E: serde::de::Error {
        Ok(Value::Boolean(value))
    }


    #[inline]
    fn visit_map<Visitor>(&mut self, mut visitor: Visitor) -> Result<Value, Visitor::Error>
    where Visitor: MapVisitor,
    {
       let mut values = HashMap::with_capacity(visitor.size_hint().0);

       while let Some((key, value)) = try!(visitor.visit()) {
           values.insert(key, value);
       }

       try!(visitor.end());

       Ok(Value::Dict(values))
   }

   #[inline]
    fn visit_seq<Visitor>(&mut self, mut visitor: Visitor) -> Result<Value, Visitor::Error>
        where Visitor: SeqVisitor,
    {
        let mut values = Vec::with_capacity(visitor.size_hint().0);;

        while let Some(value) = try!(visitor.visit()) {
            values.push(value);
        }

        try!(visitor.end());

        Ok(Value::List(values))
    }
}

/*-------------------------
         Reason
-------------------------*/

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

impl serde::de::Visitor for ReasonVisitor {
    type Value = Reason;

    #[inline]
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

/*-------------------------
       ClientRole
-------------------------*/

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

impl serde::de::Visitor for ClientRoleVisitor {
    type Value = ClientRole;

    #[inline]
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

/*-------------------------
         ErrorType
-------------------------*/

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

impl serde::de::Visitor for ErrorTypeVisitor {
    type Value = ErrorType;

    #[inline]
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


/*-------------------------
         URI
-------------------------*/

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

impl serde::de::Visitor for URIVisitor {
    type Value = URI;

    #[inline]
    fn visit_str<E>(&mut self, value: &str) -> Result<URI, E>
        where E: serde::de::Error,
    {
        Ok(URI {
            uri: value.to_string()
        })
    }

}

/*-------------------------
         RouterRole
-------------------------*/

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

impl serde::de::Visitor for RouterRoleVisitor {
    type Value = RouterRole;

    #[inline]
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
