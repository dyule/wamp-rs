use std::collections::HashMap;
use CallResult;
use serde;
use super::{Reason, CallError};

pub type Dict = HashMap<String, Value>;
pub type List = Vec<Value>;

// TODO properly implement Hash and Eq
#[derive(Debug, PartialEq, Clone, Hash, Eq)]
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

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    // The ID and URI types cannot be distinguished from string and integer types respectively.
    // So, we just ignore them here
    Dict(Dict),
    Integer(i64),
    String(String),
    List(List),
    Boolean(bool)
}

struct URIVisitor;
struct ValueVisitor;


pub trait ArgList {
    fn get_int(&self, index: usize) -> CallResult<Option<i64>>;
    fn get_string<'a>(&'a self, index: usize) -> CallResult<Option<&'a str>>;
    fn verify_len(&self, expected_len: usize) -> CallResult<()>;
}

pub trait ArgDict {
    fn get_int(&self, key: &str) -> CallResult<Option<i64>>;
    fn get_string<'a>(&'a self, key: &str) -> CallResult<Option<&'a str>>;
}

impl ArgList for List {
    fn get_int(&self, index: usize) -> CallResult<Option<i64>> {
        let value = self.get(index);
        match value {
            Some(value) => {
                if let &Value::Integer(value) = value {
                    Ok(Some(value))
                } else {
                    Err(CallError::new(Reason::InvalidArgument, Some(vec![Value::String(format!("Expected integer, got {}", value.summarize()))]), None))
                }
            },
            None => {
                Ok(None)
            }
        }
    }

    fn get_string<'a>(&'a self, index: usize) -> CallResult<Option<&'a str>> {
        let value = self.get(index);
        match value {
            Some(value) => {
                if let &Value::String(ref value) = value {
                    Ok(Some(value))
                } else {
                    Err(CallError::new(Reason::InvalidArgument, Some(vec![Value::String(format!("Expected string, got {}", value.summarize()))]), None))
                }
            },
            None => {
                Ok(None)
            }
        }
    }

    fn verify_len(&self, expected_len: usize) -> CallResult<()> {
        if self.len() >= expected_len {
            Ok(())
        } else {
            Err(CallError::new(Reason::InvalidArgument, Some(vec![Value::String(format!("Expected {} arguments, got {}", expected_len, self.len()))]), None))
        }
    }
}

impl ArgDict for Dict {
    fn get_int(&self, key: &str) -> CallResult<Option<i64>> {
        let value = self.get(key);
        match value {
            Some(value) => {
                if let &Value::Integer(value) = value {
                    Ok(Some(value))
                } else {
                    Err(CallError::new(Reason::InvalidArgument, Some(vec![Value::String(format!("Expected integer, got {}", value.summarize()))]), None))
                }
            },
            None => {
                Ok(None)
            }
        }
    }
    fn get_string<'a>(&'a self, key: &str) -> CallResult<Option<&'a str>> {
        let value = self.get(key);
        match value {
            Some(value) => {
                if let &Value::String(ref value) = value {
                    Ok(Some(value))
                } else {
                    Err(CallError::new(Reason::InvalidArgument, Some(vec![Value::String(format!("Expected string, got {}", value.summarize()))]), None))
                }
            },
            None => {
                Ok(None)
            }
        }
    }
}

impl Value {
    pub fn summarize(&self) -> String {
        match self {
            &Value::Dict(ref d) => {
                let mut result = String::new();
                result.push('{');
                result.push_str(&d.iter().take(50).map(|(key, value)| format!("{}:{}", key, value.summarize())).collect::<Vec<_>>().join(","));
                result.push('}');
                result
            },
            &Value::Integer(i) => {
                i.to_string()
            },
            &Value::String(ref s) => {
                if s.len() > 50 {
                    s[..50].to_string()
                } else {
                    s.clone()
                }
            }
            &Value::List(ref l) => {
                let mut result = String::new();
                result.push('[');
                result.push_str(&l.iter().take(50).map(|element| element.summarize()).collect::<Vec<_>>().join(","));
                result.push(']');
                result
            }
            &Value::Boolean(b) => {
                b.to_string()
            }
        }
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
    fn visit_i64<E>(&mut self, value: i64) -> Result<Value, E>
    where E: serde::de::Error {
        Ok(Value::Integer(value))
    }

    #[inline]
    fn visit_u64<E>(&mut self, value: u64) -> Result<Value, E>
    where E: serde::de::Error {
        Ok(Value::Integer(value as i64))
    }

    #[inline]
    fn visit_bool<E>(&mut self, value: bool) -> Result<Value, E>
    where E: serde::de::Error {
        Ok(Value::Boolean(value))
    }


    #[inline]
    fn visit_map<Visitor>(&mut self, mut visitor: Visitor) -> Result<Value, Visitor::Error>
    where Visitor: serde::de::MapVisitor,
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
        where Visitor: serde::de::SeqVisitor,
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
         Value
-------------------------*/
impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::ser::Serializer,
    {
        match *self {
            Value::Dict(ref dict) => dict.serialize(serializer),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Integer(i) => serializer.serialize_i64(i),
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

/*-------------------------
         URI
-------------------------*/

impl serde::Serialize for URI {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::ser::Serializer,
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
