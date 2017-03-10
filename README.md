WAMP-RS
=======

WAMP-RS is a Rust implementation of the
[Web Application Messaging Protcol (WAMP)](http://wamp-proto.org/).

At present the entire Basic Profile is supported, as well as pattern based subscriptions and registrations from the Advanced Profile.

There is currently no support for secure connections.

For instructions on how to use, please see the [examples](examples) directory.

To include in your project, place the following in your `Cargo.toml`

```toml
[dependencies]
wamp = "0.1"
```

WAMP-RS uses [serde-rs](https://github.com/serde-rs/serde), which requires Rust 1.15 or greater.
