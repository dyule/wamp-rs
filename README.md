WAMP-RS
=======

WAMP-RS is a Rust implementation of the
[Web Application Messaging Protcol (WAMP)](http://wamp-proto.org/).

It is a work in progress, so as of yet, only the publisher, subscriber and broker
roles have been implemented (and no advanced profile).  You can see an example
of how to use them in the examples folder.

Building on Windows
-------------------
WAMP-RS is dependent on
[Rust-WebSocket](https://github.com/cyderize/rust-websocket), which is
dependent on [rust-openssl](https://github.com/sfackler/rust-openssl).  In order
to build rust-openssl (and therefore WAMP-RS) on Windows, take a look a the
instructions at [https://github.com/sfackler/rust-openssl#windows]
