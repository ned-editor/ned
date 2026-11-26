//! ned_rope: chunked b-tree text buffer with line indexing for ned editor.

#![warn(missing_docs)]

pub mod rope_buffer;
pub mod tree_metric;
pub mod tree_node;

pub use rope_buffer::rope;
pub use tree_metric::metric;
pub use tree_node::node;
