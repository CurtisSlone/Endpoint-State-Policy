//! ESP Language Compiler
//!
//! A multi-pass compiler for ESP (Endpoint State Policy) files designed for
//! integration into compliance scanners.

// ============================================================================
// PUBLIC API - High-Level Interface
// ============================================================================

/// High-level pipeline API for processing ESP files
pub mod pipeline;

/// Batch directory processing with parallel execution
pub mod batch;

/// Configuration system (compile-time constants + runtime preferences)
pub mod config;

// ============================================================================
// PUBLIC API - Core Types (Re-exports)
// ============================================================================

// Pipeline types
pub use pipeline::{
    PipelineError,
    PipelineResult,
    process_file,
    process_file_with_preferences,
    PipelineOutput,
};

// Batch processing types
pub use batch::{
    BatchConfig,
    BatchError,
    BatchResults,
    process_directory,
    process_directory_with_config,
};

// ============================================================================
// PUBLIC API - AST & Symbol Access (for scanner integration)
// ============================================================================

/// Grammar and AST definitions
pub mod grammar {
    pub use crate::grammar::ast;
    pub use crate::grammar::keywords;

    // Re-export commonly used AST nodes
    pub use ast::nodes::{
        EspFile,
        DefinitionNode,
        StateDefinition,
        ObjectDefinition,
        CriteriaNode,
        CriterionNode,
        VariableDeclaration,
        // ... other key node types scanners need
    };
}

/// Symbol table and discovery results
pub mod symbols {
    pub use crate::symbols::{
        SymbolDiscoveryResult,
        GlobalSymbolTable,
        LocalSymbolTable,
        VariableSymbol,
        StateSymbol,
        ObjectSymbol,
    };
}

/// Reference validation results
pub mod references {
    pub use crate::reference_resolution::{
        ReferenceValidationResult,
    };
}

/// Semantic analysis results
pub mod semantic {
    pub use crate::semantic_analysis::{
        SemanticOutput,
        SemanticError,
    };
}

/// Structural validation results
pub mod structural {
    pub use crate::validation::{
        StructuralValidationResult,
    };
}

// ============================================================================
// PUBLIC API - Error Types (for error handling)
// ============================================================================

pub mod error {
    pub use crate::pipeline::PipelineError;
    pub use crate::batch::BatchError;
    pub use crate::file_processor::FileProcessorError;
    pub use crate::lexical::LexerError;
    pub use crate::syntax::SyntaxError;
    pub use crate::symbols::SymbolDiscoveryError;
    pub use crate::reference_resolution::ReferenceValidationError;
    pub use crate::semantic_analysis::SemanticError;
    pub use crate::validation::StructuralError;
}

// ============================================================================
// PUBLIC API - Utility Types (for span information, etc.)
// ============================================================================

pub mod utils {
    pub use crate::utils::{
        Span,
        Position,
        Spanned,
    };
}

// ============================================================================
// INTERNAL MODULES (Not part of public API)
// ============================================================================

mod file_processor;
mod lexical;
#[macro_use]
mod logging;
mod reference_resolution;
mod semantic_analysis;
mod syntax;
mod tokens;
mod validation;

// Note: grammar, symbols are partially exposed above
// These internal paths remain private
use crate::grammar as grammar_internal;
use crate::symbols as symbols_internal;

// ============================================================================
// LIBRARY INFORMATION
// ============================================================================

/// ESP Language specification version
pub const LANGUAGE_VERSION: &str = "1.0.0";

/// Parser implementation version
pub const PARSER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library capability information
#[derive(Debug, Clone)]
pub struct LibraryInfo {
    pub language_version: &'static str,
    pub parser_version: &'static str,
    pub pipeline_stages: u8,
    pub supports_parallel_processing: bool,
}

/// Get library information
pub fn get_library_info() -> LibraryInfo {
    LibraryInfo {
        language_version: LANGUAGE_VERSION,
        parser_version: PARSER_VERSION,
        pipeline_stages: 7,
        supports_parallel_processing: true,
    }
}

impl LibraryInfo {
    pub fn summary(&self) -> String {
        format!(
            "ESP Language Parser v{} (Language: v{})\n\
             Pipeline: {} stages, Parallel: {}",
            self.parser_version,
            self.language_version,
            self.pipeline_stages,
            self.supports_parallel_processing
        )
    }
}

// ============================================================================
// INITIALIZATION
// ============================================================================

/// Initialize the ESP parser library
///
/// Call this once at application startup to initialize logging and validate configuration.
pub fn init() -> Result<(), String> {
    logging::init_global_logging()?;
    pipeline::validate_pipeline()?;
    Ok(())
}

/// Validate parser integrity
pub fn validate() -> Result<(), String> {
    file_processor::init_file_processor_logging()?;
    lexical::init_lexical_analysis_logging()?;
    syntax::init_syntax_logging()?;
    symbols_internal::init_symbol_discovery_logging()?;
    reference_resolution::init_reference_validation()?;
    semantic_analysis::init_semantic_analysis_logging()?;
    validation::init_structural_validation_logging()?;
    Ok(())
}
