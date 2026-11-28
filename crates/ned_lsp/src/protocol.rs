//! Language Server Protocol (LSP) data types.
//!
//! Maps standard LSP objects to `snake_case` Rust structures with automatic
//! camelCase serialization matching the LSP JSON specification.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A position in a text document expressed as 0-based line and character offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct position {
    /// 0-based line index.
    pub line: u32,
    /// 0-based character offset within the line.
    pub character: u32,
}

impl position {
    /// Creates a new position.
    #[must_use]
    pub const fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// A range in a text document expressed as (start, end) positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct range {
    /// Range start position.
    pub start: position,
    /// Range end position.
    pub end: position,
}

impl range {
    /// Creates a new range.
    #[must_use]
    pub const fn new(start: position, end: position) -> Self {
        Self { start, end }
    }
}

/// Represents a location inside a resource, such as a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct location {
    /// Target document URI.
    pub uri: String,
    /// Target range within document.
    pub range: range,
}

/// Reference to a text document via its URI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct text_document_identifier {
    /// Target document URI.
    pub uri: String,
}

impl text_document_identifier {
    /// Creates a new text document identifier.
    #[must_use]
    pub fn new(uri: impl Into<String>) -> Self {
        Self { uri: uri.into() }
    }
}

/// Reference to a versioned text document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct versioned_text_document_identifier {
    /// Target document URI.
    pub uri: String,
    /// Monotonically increasing document version.
    pub version: i32,
}

/// An open text document item transferred from editor to language server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct text_document_item {
    /// Target document URI.
    pub uri: String,
    /// Language identifier (e.g. "rust", "python", "go").
    pub language_id: String,
    /// Document version.
    pub version: i32,
    /// Full text content of the document.
    pub text: String,
}

/// Event describing an incremental or full content change in a text document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct text_document_content_change_event {
    /// Optional range that got replaced (None indicates full document replacement).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<range>,
    /// New replacement text content.
    pub text: String,
}

/// Parameters sent with `textDocument/didOpen`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct did_open_text_document_params {
    /// The opened text document.
    pub text_document: text_document_item,
}

/// Parameters sent with `textDocument/didChange`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct did_change_text_document_params {
    /// Target versioned document.
    pub text_document: versioned_text_document_identifier,
    /// Array of content change events.
    pub content_changes: Vec<text_document_content_change_event>,
}

/// Parameters sent with `textDocument/didSave`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct did_save_text_document_params {
    /// The saved text document.
    pub text_document: text_document_identifier,
    /// Optional saved text content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Parameters sent with `textDocument/didClose`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct did_close_text_document_params {
    /// The closed text document.
    pub text_document: text_document_identifier,
}

/// Diagnostic severity level.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum diagnostic_severity {
    /// Reports an error (1).
    error = 1,
    /// Reports a warning (2).
    warning = 2,
    /// Reports an informational message (3).
    information = 3,
    /// Reports a hint (4).
    hint = 4,
}

impl From<u8> for diagnostic_severity {
    fn from(val: u8) -> Self {
        match val {
            1 => Self::error,
            2 => Self::warning,
            3 => Self::information,
            _ => Self::hint,
        }
    }
}

impl From<diagnostic_severity> for u8 {
    fn from(val: diagnostic_severity) -> Self {
        val as Self
    }
}

impl Serialize for diagnostic_severity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for diagnostic_severity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let val: u8 = Deserialize::deserialize(deserializer)?;
        Ok(Self::from(val))
    }
}

/// A diagnostic message (such as a compiler error, warning, or lint).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct diagnostic {
    /// The range to which the diagnostic applies.
    pub range: range,
    /// Severity level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<diagnostic_severity>,
    /// Diagnostic code or rule identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<Value>,
    /// Source or tool that produced the diagnostic (e.g. "rustc").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Diagnostic description.
    pub message: String,
}

/// Parameters sent from server with `textDocument/publishDiagnostics`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct publish_diagnostics_params {
    /// Document URI for which diagnostics are reported.
    pub uri: String,
    /// Optional document version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i32>,
    /// List of diagnostics.
    pub diagnostics: Vec<diagnostic>,
}

/// Information about the client editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct client_info {
    /// Editor name ("ned").
    pub name: String,
    /// Editor version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Parameters sent with the `initialize` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct initialize_params {
    /// Process ID of the client editor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    /// Root URI of the opened workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_uri: Option<String>,
    /// Client capabilities.
    pub capabilities: Value,
    /// Information about client editor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_info: Option<client_info>,
}

/// Parameters sent for position-based queries (e.g. definition, hover).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct text_document_position_params {
    /// Target document identifier.
    pub text_document: text_document_identifier,
    /// Target position.
    pub position: position,
}

impl text_document_position_params {
    /// Creates new position parameters.
    #[must_use]
    pub fn new(uri: impl Into<String>, line: u32, character: u32) -> Self {
        Self {
            text_document: text_document_identifier::new(uri),
            position: position::new(line, character),
        }
    }
}

/// Result of a hover query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct hover {
    /// Hover contents (typically markdown or plaintext).
    pub contents: Value,
    /// Optional range the hover applies to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<range>,
}

/// A completion item proposed by the language server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct completion_item {
    /// Human-readable label of this completion item.
    pub label: String,
    /// Human-readable string with additional detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// String to be inserted into the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_text: Option<String>,
}
