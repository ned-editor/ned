//! Recursive TOML document merging with array-by-name matching.

use toml::Value;

/// Merges two TOML values, with properties from `right` overriding `left`.
///
/// - For Tables: recursively merges matching keys up to `depth`, unioning non-matching keys.
/// - For Arrays of Tables containing a `"name"` property (such as `[[language]]` entries):
///   matches entries by their name string and merges them individually.
/// - For all other types: `right` replaces `left`.
#[must_use]
pub fn merge_toml(left: Value, right: Value, depth: usize) -> Value {
    if depth == 0 {
        return right;
    }

    match (left, right) {
        (Value::Table(mut left_table), Value::Table(right_table)) => {
            for (key, right_val) in right_table {
                match left_table.remove(&key) {
                    Some(left_val) => {
                        let merged = merge_toml(left_val, right_val, depth - 1);
                        left_table.insert(key, merged);
                    }
                    None => {
                        left_table.insert(key, right_val);
                    }
                }
            }
            Value::Table(left_table)
        }
        (Value::Array(mut left_items), Value::Array(right_items)) => {
            fn item_name(v: &Value) -> Option<&str> {
                v.get("name").and_then(Value::as_str)
            }

            // Check if right items have a "name" key indicating named object list
            let has_named_items = right_items.iter().any(|item| item_name(item).is_some());

            if has_named_items {
                for r_item in right_items {
                    if let Some(r_name) = item_name(&r_item) {
                        let match_pos = left_items
                            .iter()
                            .position(|l_item| item_name(l_item) == Some(r_name));

                        if let Some(pos) = match_pos {
                            let l_item = left_items.remove(pos);
                            let merged = merge_toml(l_item, r_item, depth - 1);
                            left_items.insert(pos, merged);
                        } else {
                            left_items.push(r_item);
                        }
                    } else {
                        left_items.push(r_item);
                    }
                }
                Value::Array(left_items)
            } else {
                Value::Array(right_items)
            }
        }
        (_, right_val) => right_val,
    }
}
