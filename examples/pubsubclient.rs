extern crate wamp;
extern crate eventual;
extern crate serde;
use wamp::client::{Connection, Client, Subscription};
use wamp::{URI, Dict, List, Value};
use std::env;
use std::collections::HashMap;
use std::thread::{current, park};
use std::io;
use std::sync::{Mutex, Arc};
use eventual::Async;

#[macro_use]
extern crate log;

use log::{LogRecord, LogLevel, LogMetadata};

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LogLevel::Debug
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) && record.location().module_path().starts_with("wamp") {
            println!("{} - {} ", record.level(), record.args());
        }
    }
}

enum Command {
    Sub,
    Pub,
    Unsub,
    List,
    Quit,
    NoOp,
    Invalid(String)
}

fn print_prompt() {
    println!("Enter a command");
}

fn get_input_from_user() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input
}

fn process_input(input: String) -> (Command, Vec<String>) {
    let mut i_iter = input.splitn(2, ' ');
    let command = match i_iter.next() {
        Some(command) => command.trim().to_lowercase(),
        None          => return (Command::NoOp, Vec::new())
    };
    let command = match command.as_str() {
        "pub" => Command::Pub,
        "sub" => Command::Sub,
        "unsub" => Command::Unsub,
        "list" => Command::List,
        "quit" => Command::Quit,
        "" => Command::NoOp,
        x => Command::Invalid(x.to_string())
    };
    let args = match i_iter.next() {
        Some(args_string) => args_string.split(',').map(|s| s.trim().to_string()).collect(),
        None              => Vec::new()
    };
    (command, args)
}

fn subscribe(client: &mut Client, subscriptions: &mut Arc<Mutex<Vec<Subscription>>>, args: Vec<String>) {
    if args.len() > 1 {
        println!("Too many arguments to subscribe.  Ignoring");
    } else if args.len() == 0 {
        println!("Please specify the topic to subscribe to");
        return;
    }
    let topic = args[0].clone();
    let mut subscriptions = subscriptions.clone();
    client.subscribe(URI::new(&topic), Box::new(move |args, kwargs|{
        println!("Recieved message on topic {} with args {:?} and kwargs {:?}", topic, args, kwargs);
    })).unwrap().and_then(move |subscription|{
        println!("Subscribed to topic {}", subscription.topic.uri);
        subscriptions.lock().unwrap().push(subscription);
        Ok(())
    }).await();
}

fn unsubscribe(client: &mut Client, subscriptions: &mut Arc<Mutex<Vec<Subscription>>>, args: Vec<String>) {
    if args.len() > 1 {
        println!("Too many arguments to subscribe.  Ignoring");
    } else if args.len() == 0 {
        println!("Please specify the topic to subscribe to");
        return;
    }
    let topic = match args[0].parse::<usize>() {
        Ok(i) => {
            let mut subscriptions = subscriptions.lock().unwrap();
            if i >= subscriptions.len() {
                println!("Invalid subscription index: {}", i);
                return;
            }
            let subscription = subscriptions.remove(i);
            let topic = subscription.topic.uri.clone();
            client.unsubscribe(subscription).unwrap().and_then(move |()| {
                println!("Successfully unsubscribed from {}", topic);
                Ok(())
            });
        },
        Err(_) => {
            println!("Invalid subscription index: {}", args[0]);
        }
    };
}

fn list(subscriptions: &mut Arc<Mutex<Vec<Subscription>>>) {
    let subscriptions = subscriptions.lock().unwrap();
    for (index, subscription) in subscriptions.iter().enumerate() {
        println!("{} {}", index, subscription.topic.uri);
    }
}

fn publish(client: &mut Client, args: Vec<String>) {
    if args.len() == 0 {
        println!("Please specify a topic to publish to");
    }
    let mut topic_arr = args.clone();
    let args = topic_arr.split_off(1);
    let args = args.iter().map(|arg|{
        match arg.parse::<u64>() {
            Ok(i)  => Value::Integer(i),
            Err(_) => Value::String(arg.clone())
        }
    }).collect();
    client.publish(URI::new(&topic_arr[0]), Some(args), None);
}

fn event_loop(mut client: Client) {
    let mut subscriptions = Arc::new(Mutex::new(Vec::new()));
    loop {
        print_prompt();
        let input = get_input_from_user();
        let (command, args) = process_input(input);
        match command {
            Command::Sub => subscribe(&mut client, &mut subscriptions, args),
            Command::Pub => publish(&mut client, args),
            Command::Unsub => unsubscribe(&mut client, &mut subscriptions, args),
            Command::List => list(&mut subscriptions),
            Command::Quit => break,
            Command::NoOp => {},
            Command::Invalid(bad_command) => print!("Invalid command: {}", bad_command)
        }
    }
    client.shutdown();

}


fn main() {
    log::set_logger(|max_log_level| {
        max_log_level.set(log::LogLevelFilter::Debug);
        Box::new(SimpleLogger)
    }).unwrap();
    let connection = Connection::new("ws://127.0.0.1:8090/ws", "realm1");
    info!("Connecting");
    let mut client = connection.connect().unwrap();
    // let main_thread = current();
    // let published = move |topic:&URI| {
    //     //main_thread.unpark()
    // };
    info!("Connected");
    event_loop(client);
    // let mut args = env::args();
    // let _ = args.next().unwrap();
    // let flag = args.next().unwrap();
    // if flag == "sub" {
    //     info!("Subscribing");
    //     client.subscribe(URI::new("ca.dal.test.topic1"), Box::new(callback)).unwrap();
    //     info!("Waiting");
    //     loop {
    //
    //     }
    //
    // } else if flag == "pub" {
    //     info!("Sending");
    //     client.on_published(Box::new(published));
    //     client.publish(URI::new("ca.dal.test.topic1"), Some(vec![Value::Integer(5)]), None).unwrap();
    //     info!("Sent");
    //     park()
    // }



}

fn callback(args: List, kwargs: Dict) {
    info!("args: {:?}, kwargs: {:?}", args, kwargs);
}
