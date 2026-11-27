//! Prefix key dispatch tree and trie node structures.

use std::collections::HashMap;

use super::action;
use super::key_types::key_event;

/// A node or terminal action in the prefix key dispatch trie.
#[derive(Debug, Clone, PartialEq)]
pub enum key_trie {
    /// Terminal action to execute.
    action(action),
    /// Sequence of actions to execute sequentially.
    sequence(Vec<action>),
    /// Sub-trie containing nested prefix keys.
    node(key_trie_node),
}

/// Interior node in the keymap prefix tree.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct key_trie_node {
    /// Human-readable label for which-key hints.
    pub label: String,
    /// Mapping of key event to sub-trie or action.
    pub map: HashMap<key_event, key_trie>,
}

impl key_trie_node {
    /// Creates a new trie node.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            map: HashMap::new(),
        }
    }

    /// Inserts a key-action binding.
    pub fn insert(&mut self, key_str: &str, act: action) {
        if let Some(ev) = key_event::parse(key_str) {
            self.map.insert(ev, key_trie::action(act));
        }
    }

    /// Inserts a nested sub-node (prefix key).
    pub fn insert_node(&mut self, key_str: &str, sub_node: Self) {
        if let Some(ev) = key_event::parse(key_str) {
            self.map.insert(ev, key_trie::node(sub_node));
        }
    }

    /// Looks up a key event in this node.
    #[must_use]
    pub fn get(&self, key: &key_event) -> Option<&key_trie> {
        self.map.get(key)
    }

    /// Deep merges another trie node into this one, with `other` taking priority.
    pub fn merge(&mut self, other: Self) {
        for (key, other_trie) in other.map {
            match (self.map.get_mut(&key), other_trie) {
                (Some(key_trie::node(curr_node)), key_trie::node(new_node)) => {
                    curr_node.merge(new_node);
                }
                (_, new_trie) => {
                    self.map.insert(key, new_trie);
                }
            }
        }
    }
}
