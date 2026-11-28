//! Register storage for yanking, pasting, and clipboard integration.

use std::collections::HashMap;

/// Maximum number of named registers allowed.
const MAX_NAMED_REGISTERS: usize = 128;
/// Maximum bytes allowed per register (4 MiB).
const MAX_REGISTER_BYTES: usize = 4 * 1024 * 1024;

/// Storage managing text registers for yank, delete, change, and paste operations.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct register_storage {
    /// Named registers mapping character to list of selection string fragments.
    registers: HashMap<char, Vec<String>>,
    /// Last regex pattern used in search operations (`/`).
    pub last_search: Option<String>,
    /// Last typable command executed in command mode (`:`).
    pub last_command: Option<String>,
}

impl register_storage {
    /// Creates a new empty register storage.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registers: HashMap::new(),
            last_search: None,
            last_command: None,
        }
    }

    /// Reads content from register `reg`.
    ///
    /// Register `_` (blackhole) always returns `None`.
    #[must_use]
    pub fn read(&self, reg: char) -> Option<Vec<String>> {
        match reg {
            '_' => None,
            '+' => crate::clipboard::read(crate::clipboard::clipboard_target::clipboard).ok(),
            '*' => crate::clipboard::read(crate::clipboard::clipboard_target::primary).ok(),
            '/' => self.last_search.as_ref().map(|s| vec![s.clone()]),
            ':' => self.last_command.as_ref().map(|c| vec![c.clone()]),
            _ => self.registers.get(&reg).cloned(),
        }
    }

    /// Writes values into register `reg`.
    ///
    /// Handles uppercase append (e.g. `'A'` appends to `'a'`), blackhole discard (`_`),
    /// and rejects writes to read-only special registers (`#`, `.`, `%`).
    ///
    /// # Errors
    ///
    /// Returns an error if the register is read-only, exceeds 4 MiB, or exceeds register capacity.
    pub fn write(&mut self, reg: char, values: Vec<String>) -> Result<(), String> {
        match reg {
            '_' => Ok(()), // Blackhole: silently discard writes
            '#' | '.' | '%' => Err(format!("Register \"{reg}\" is read-only")),
            '+' => crate::clipboard::write(crate::clipboard::clipboard_target::clipboard, &values),
            '*' => crate::clipboard::write(crate::clipboard::clipboard_target::primary, &values),
            '/' => {
                if let Some(first) = values.into_iter().next() {
                    self.last_search = Some(first);
                }
                Ok(())
            }
            ':' => {
                if let Some(first) = values.into_iter().next() {
                    self.last_command = Some(first);
                }
                Ok(())
            }
            c if c.is_ascii_uppercase() => {
                let lower = c.to_ascii_lowercase();
                if self.registers.len() >= MAX_NAMED_REGISTERS
                    && !self.registers.contains_key(&lower)
                {
                    return Err(format!(
                        "Maximum register limit reached ({MAX_NAMED_REGISTERS})"
                    ));
                }
                let total_new_bytes: usize = values.iter().map(String::len).sum();
                let entry = self.registers.entry(lower).or_default();
                let current_bytes: usize = entry.iter().map(String::len).sum();
                if current_bytes + total_new_bytes > MAX_REGISTER_BYTES {
                    return Err("Register content exceeds 4 MiB limit".to_string());
                }
                entry.extend(values);
                Ok(())
            }
            c => {
                if self.registers.len() >= MAX_NAMED_REGISTERS && !self.registers.contains_key(&c) {
                    return Err(format!(
                        "Maximum register limit reached ({MAX_NAMED_REGISTERS})"
                    ));
                }
                let total_bytes: usize = values.iter().map(String::len).sum();
                if total_bytes > MAX_REGISTER_BYTES {
                    return Err("Register content exceeds 4 MiB limit".to_string());
                }
                self.registers.insert(c, values);
                Ok(())
            }
        }
    }

    /// Reads from the default unnamed register (`"`).
    #[must_use]
    pub fn read_default(&self) -> Option<Vec<String>> {
        self.read('"')
    }

    /// Writes values to the default unnamed register (`"`).
    pub fn write_default(&mut self, values: Vec<String>) {
        let _ = self.write('"', values);
    }
}
