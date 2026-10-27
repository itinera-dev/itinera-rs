#![cfg_attr(feature = "macros", doc = include_str!("../README.md"))]
#![cfg_attr(
    not(feature = "macros"),
    doc = "Itinera is a workflow framework that keeps business rules separate from flow control."
)]
#![forbid(unsafe_code)]

pub use itinera_core::*;
