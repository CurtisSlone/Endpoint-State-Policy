//! # Scan Results Module
//!
//! This module provides types and utilities for ESP compliance validation results.
//! It handles the generation, serialization, and transformation of scan outcomes
//! for integration with SIEM/SOAR tools and compliance reporting systems.
//!
//! ## Core Types
//! - [`ScanResult`] - Complete scan result for one ESP definition
//! - [`ComplianceFinding`] - Individual compliance violations
//! - [`ScanMetadata`] - Metadata about scan execution and ESP definition
//! - [`ResultGenerationError`] - Errors that occur during result processing
//!
//! ## Usage
//! ```rust
//! use esp_scanner_base::results::{ScanResult, EspMetadata, HostContext, UserContext};
//!
//! let esp_metadata = EspMetadata::default_test();
//! let scan_result = ScanResult::new(
//!     "scan-001".to_string(),
//!     esp_metadata,
//!     HostContext::from_system(),
//!     UserContext::from_environment(),
//! );
//! ```

pub mod error;
pub mod generator;
pub mod types;

// Re-export all public types for convenient access
pub use error::*;
pub use generator::ResultGenerator;
pub use types::*;

// Future module stubs for planned functionality
// pub mod formatters;  // Output format conversion (XML, CSV, etc.)
// pub mod exporters;   // SIEM/SOAR tool integrations
// pub mod aggregators; // Multi-scan result consolidation
