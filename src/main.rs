#![cfg_attr(feature = "serde_macros", feature(custom_derive, plugin))]
#![cfg_attr(feature = "serde_macros", plugin(serde_macros))]
#![feature(braced_empty_structs)]

extern crate serde;
extern crate serde_json;
extern crate websocket;

mod messages;

fn main() {

}
