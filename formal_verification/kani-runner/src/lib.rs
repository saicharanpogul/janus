//! Janus Kani harness runner.
//!
//! The actual harnesses live under `formal_verification/<program>/kani.rs`
//! and are pulled in by `tests/<program>.rs` files via `include!`. This
//! lib is intentionally empty; everything is in the test binaries so
//! `cargo kani --tests --harness <name>` resolves correctly.
