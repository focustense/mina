# mina-core

Macros for [`mina`](https://docs.rs/mina/).

This is an internal crate that exists to satisfy the requirement for procedural macros to be in their own crate.

Apps shouldn't use this crate directly; use [`mina`](https://docs.rs/mina/) instead, which re-exports all relevant
macros from this crate.