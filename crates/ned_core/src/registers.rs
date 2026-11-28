//! Register storage for yanking, pasting, and clipboard integration.

use std::collections::HashMap;

/// Storage managing text registers for yank and paste operations.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct register_storage {
    registers: HashMap<char, Vec<String>>,
}

impl register_storage {
    /// Creates a new empty register storage.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registers: HashMap::new(),
        }
    }

    /// Reads content from register `reg`.
    ///
    /// Register `_` (blackhole) always returns `None`.
    #[must_use]
    pub fn read(&self, reg: char) -> Option<&[String]> {
        if reg == '_' {
            return None;
        }
        self.registers.get(&reg).map(std::vec::Vec::as_slice)
    }

    /// Writes values into register `reg`.
    ///
    /// Register `_` (blackhole) discards writes silently.
    pub fn write(&mut self, reg: char, values: Vec<String>) {
        if reg == '_' {
            return;
        }
        self.registers.insert(reg, values);
    }

    /// Reads from the default register (`"`).
    #[must_use]
    pub fn read_default(&self) -> Option<&[String]> {
        self.read('"')
    }

    /// Writes values to the default register (`"`).
    pub fn write_default(&mut self, values: Vec<String>) {
        self.write('"', values);
    }
}
