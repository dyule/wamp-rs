extern crate wamp;

use wamp::router::Router;


#[macro_use]
extern crate log;

use log::{LogRecord, LogLevel, LogMetadata};

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LogLevel::Debug
    }

    fn log(&self, record: &LogRecord) {
        // if record.location().module_path().starts_with("mio") {
        //     return
        // }
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }
}



fn main() {
    log::set_logger(|max_log_level| {
        max_log_level.set(log::LogLevelFilter::Trace);
        Box::new(SimpleLogger)
    }).unwrap();
    let mut router = Router::new();
    router.add_realm("realm1");
    info!("Router listening");
    router.listen("127.0.0.1:8090")
}
