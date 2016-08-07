extern crate wamp;
extern crate eventual;
#[macro_use]
extern crate log;
extern crate env_logger;

use wamp::client::{Connection, Client, Subscription};
use wamp::{URI, Value, MatchingPolicy, Dict, List, CallResult, Reason};
use std::io;
use std::sync::{Mutex, Arc};
use eventual::Async;
use std::collections::HashMap;
use std::slice::Iter;

fn addition_callback(args: List, _kwargs: Dict) -> CallResult<(List, Dict)> {
    let mut args_iter = args.iter();
    let a = try!(extract_int(&mut args_iter));
    let b = try!(extract_int(&mut args_iter));
    Ok((vec![Value::Integer(a + b)], HashMap::new()))
}

fn multiplication_callback(args: List, _kwargs: Dict) -> CallResult<(List, Dict)> {
    let mut args_iter = args.iter();
    let a = try!(extract_int(&mut args_iter));
    let b = try!(extract_int(&mut args_iter));
    Ok((vec![Value::Integer(a * b)], HashMap::new()))
}

fn echo_callback(args: List, kwargs: Dict) -> CallResult<(List, Dict)> {
    Ok((args, kwargs))
}

fn extract_int(arg_iter: &mut Iter<Value>) -> CallResult<u64> {
    let value = arg_iter.next();
    match value {
        Some(value) => {
            if let &Value::Integer(value) = value {
                Ok(value)
            } else {
                Err(Reason::InvalidArgument)
            }
        },
        None => {
            Err(Reason::InvalidArgument)
        }
    }
}

fn main() {
    env_logger::init().unwrap();
    let connection = Connection::new("ws://127.0.0.1:8090/ws", "realm1");
    info!("Connecting");
    let mut client = connection.connect().unwrap();

    info!("Connected");
    info!("Registering Addition Procedure");
    client.register(URI::new("com.test.add"), Box::new(addition_callback)).unwrap().await().unwrap();

    info!("Registering Multiplication Procedure");
    let mult_reg = client.register(URI::new("com.test.mult"), Box::new(multiplication_callback)).unwrap().await().unwrap();

    info!("Unregistering Multiplication Procedure");
    client.unregister(mult_reg).unwrap().await().unwrap();

    info!("Registering Echo Procedure");
    let mult_reg = client.register(URI::new("com.test.mult"), Box::new(multiplication_callback)).unwrap().await().unwrap();

    println!("Press enter to quit");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    client.shutdown().unwrap().await().unwrap();
}
