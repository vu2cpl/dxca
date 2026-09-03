//! Library surface of the server crate — exists so integration tests can
//! drive the pipeline; the `dxca` binary (main.rs) is a thin consumer.

pub mod api;
pub mod assets;
pub mod auth;
pub mod builtin;
pub mod cmdrouter;
pub mod commands;
pub mod config;
pub mod db;
pub mod health;
pub mod nodes;
pub mod pipeline;
pub mod refresh;
pub mod telnetcmd;
pub mod users;
