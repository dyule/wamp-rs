// All this is necessary because Serde decided that the best way
// to serialize an empty struct was as a null value, and the best way
// to deserialize it was as an empty list.  So instead we serialize it as a map.
macro_rules! serialize_empty {
    ($name:tt) => {
        serialize_empty_workaround!($name, $name);
    };
}

macro_rules! serialize_empty_workaround {
    ($name:ty, $obj:expr) => {
        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let struc = try!(serializer.serialize_struct("", 0));
                struc.end()
            }
        }

        impl<'de> serde::de::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<$name, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                {
                    struct Visitor;
                    impl<'de> serde::de::Visitor<'de> for Visitor {
                        type Value = $name;

                        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                            formatter.write_str("empty struct")
                        }

                        #[inline]
                        fn visit_unit<E>(self) -> Result<$name, E>
                        where
                            E: serde::de::Error,
                        {
                            Ok($obj)
                        }
                        #[inline]
                        fn visit_map<A>(self, _visitor: A) -> Result<$name, A::Error>
                        where
                            A: serde::de::MapAccess<'de>,
                        {
                            self.visit_unit()
                        }
                    }
                    deserializer.deserialize_unit_struct("", Visitor)
                }
            }
        }
    };
}

mod error;
mod options;
mod roles;
mod value;

use serde;
use std::fmt;

pub use messages::types::error::*;
pub use messages::types::options::*;
pub use messages::types::roles::*;
pub use messages::types::value::*;

fn is_not(b: &bool) -> bool {
    !*b
}

/**************************
         Structs
**************************/

/// The policies that can be used for matching a uri pattern.
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum MatchingPolicy {
    /// The given pattern matches any URI that has it as a prefix
    Prefix,
    /// The given pattern contains at least one 'wildcard' segment which can match any segment at the same location
    Wildcard,
    /// The given pattern only matches URIs that are identical.
    Strict,
}

/// The policies that dictate how invocations are distributed amongst shared registrations
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum InvocationPolicy {
    // Only one reigistration per uri (the default)
    Single,
    // Callee selcted sequentially from the list of registrants
    RoundRobin,
    // Callee selcted randomly from the list of registrants
    Random,
    // First callee (in orer of registration) is called
    First,
    // Last callee (in order of registration( is called
    Last,
}

/**************************
        Visitors
**************************/

struct MatchingPolicyVisitor;
struct InvocationPolicyVisitor;

impl MatchingPolicy {
    #[inline]
    fn is_strict(&self) -> bool {
        self == &MatchingPolicy::Strict
    }
}

impl InvocationPolicy {
    #[inline]
    fn is_single(&self) -> bool {
        self == &InvocationPolicy::Single
    }
}

impl Default for MatchingPolicy {
    #[inline]
    fn default() -> MatchingPolicy {
        MatchingPolicy::Strict
    }
}

impl Default for InvocationPolicy {
    #[inline]
    fn default() -> InvocationPolicy {
        InvocationPolicy::Single
    }
}

/*-------------------------
       MatchingPolicy
-------------------------*/

impl serde::Serialize for MatchingPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let ser_str = match *self {
            MatchingPolicy::Prefix => "prefix",
            MatchingPolicy::Wildcard => "wildcard",
            MatchingPolicy::Strict => "",
        };
        serializer.serialize_str(ser_str)
    }
}

impl<'de> serde::Deserialize<'de> for MatchingPolicy {
    fn deserialize<D>(deserializer: D) -> Result<MatchingPolicy, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(MatchingPolicyVisitor)
    }
}

impl<'de> serde::de::Visitor<'de> for MatchingPolicyVisitor {
    type Value = MatchingPolicy;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("matching policy for registration")
    }

    #[inline]
    fn visit_str<E>(self, value: &str) -> Result<MatchingPolicy, E>
    where
        E: serde::de::Error,
    {
        match value {
            "prefix" => Ok(MatchingPolicy::Prefix),
            "wildcard" => Ok(MatchingPolicy::Wildcard),
            x => Err(serde::de::Error::custom(format!(
                "Invalid matching policy: {}",
                x
            ))),
        }
    }
}

impl serde::Serialize for InvocationPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let ser_str = match *self {
            InvocationPolicy::Single => "single",
            InvocationPolicy::RoundRobin => "roundrobin",
            InvocationPolicy::Random => "random",
            InvocationPolicy::First => "first",
            InvocationPolicy::Last => "last",
        };
        serializer.serialize_str(ser_str)
    }
}

impl<'de> serde::Deserialize<'de> for InvocationPolicy {
    fn deserialize<D>(deserializer: D) -> Result<InvocationPolicy, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(InvocationPolicyVisitor)
    }
}

impl<'de> serde::de::Visitor<'de> for InvocationPolicyVisitor {
    type Value = InvocationPolicy;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("invocation policy for a procedure")
    }

    #[inline]
    fn visit_str<E>(self, value: &str) -> Result<InvocationPolicy, E>
    where
        E: serde::de::Error,
    {
        match value {
            "single" => Ok(InvocationPolicy::Single),
            "roundrobin" => Ok(InvocationPolicy::RoundRobin),
            "random" => Ok(InvocationPolicy::Random),
            "first" => Ok(InvocationPolicy::First),
            "last" => Ok(InvocationPolicy::Last),
            x => Err(serde::de::Error::custom(format!(
                "Invalid invocation policy: {}",
                x
            ))),
        }
    }
}
