#[cfg(feature = "serde_macros")]
include!("types.rs.in");

#[cfg(not(feature = "serde_macros"))]
include!(concat!(env!("OUT_DIR"), "/messages/types.rs"));
