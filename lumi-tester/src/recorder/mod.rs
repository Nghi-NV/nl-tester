//! Recorder module for capturing user actions and generating YAML test scripts
//!
//! This module provides:
//! - Smart selector scoring to choose the best element selector
//! - Event recording to capture user interactions
//! - YAML generation to output recorded actions

pub mod event_recorder;
pub mod selector_scorer;
pub mod yaml_generator;

pub use event_recorder::EventRecorder;
pub use selector_scorer::{SelectorCandidate, SelectorScorer};
pub use yaml_generator::YamlGenerator;
