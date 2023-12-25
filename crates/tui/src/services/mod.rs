use crate::utils::dispatch::{ActionHandler, EventHandler};

pub mod crossterm;
pub mod midi;
pub mod screens;

pub trait Service: ActionHandler + EventHandler + Send + Sync {}
