extern crate wamp;
extern crate serde;
use wamp::client::Connection;
use wamp::{URI, Dict, List, Value};
use std::env;
use std::collections::HashMap;

#[macro_use]
extern crate log;

use log::{LogRecord, LogLevel, LogMetadata};

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LogLevel::Debug
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }
}



fn main() {
    log::set_logger(|max_log_level| {
        max_log_level.set(log::LogLevelFilter::Debug);
        Box::new(SimpleLogger)
    }).unwrap();
    let connection = Connection::new("ws://127.0.0.1:8090/ws", "realm1");
    info!("Connecting");
    let mut client = connection.connect().unwrap();
    info!("Connected");
    let mut args = env::args();
    let _ = args.next().unwrap();
    let flag = args.next().unwrap();
    if flag == "sub" {
        info!("Subscribing");
        client.subscribe(URI::new("ca.dal.test.topic1"), Box::new(callback)).unwrap();
        info!("Waiting");
        loop {

        }

    } else if flag == "pub" {
        info!("Sending");
        client.publish(URI::new("ca.dal.test.topic1"), vec![Value::Integer(5)], HashMap::new()).unwrap();
        info!("Sent");
        loop {
            
        }
    }



}

fn callback(args: List, kwargs: Dict) {
    info!("args: {:?}, kwargs: {:?}", args, kwargs);
}
