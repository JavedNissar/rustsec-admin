//! `rustsec-admin` CLI application
//!
//! Administrative tool for the RustSec Advisory Database

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

pub mod application;
pub mod commands;
pub mod config;
pub mod error;
pub mod linter;
pub mod prelude;
pub mod pull_request;
pub mod web;
