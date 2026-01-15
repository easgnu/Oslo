//! Oslo network node CLI library.
#![warn(missing_docs)]

mod chain_spec;
mod client;
mod cli;
mod command;
mod rpc;
#[macro_use]
mod service;

fn main() -> sc_cli::Result<()> { command::run()}