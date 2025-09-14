//! Core module  
//! 
//! This module contains the core application structure including:
//! - HostedObject trait and implementation
//! - Application initialization logic
//! - Main application runtime

pub mod hosted_object;

// Re-export the main application components
pub use hosted_object::{HostedObject, HostedObjectTrait};