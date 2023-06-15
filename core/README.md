# mina-core

Core types and traits for [`mina`](https://docs.rs/mina/).

This is an internal library crate that exists to share code between the main library and proc macros.

Apps shouldn't use this crate directly; use [`mina`](https://docs.rs/mina/) instead, which re-exports all relevant types
from this crate.