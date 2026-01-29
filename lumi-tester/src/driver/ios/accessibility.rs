//! iOS Accessibility hierarchy parser
//!
//! Parses the output from `idb ui describe-all` to find and match UI elements.

use anyhow::Result;
use serde::Deserialize;

/// Represents a frame/bounds of an iOS UI element
#[derive(Debug, Clone, Deserialize, Default)]
pub struct IosFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl IosFrame {
    /// Get the center point of the frame
    pub fn center(&self) -> (i32, i32) {
        let cx = self.x + self.width / 2.0;
        let cy = self.y + self.height / 2.0;
        (cx as i32, cy as i32)
    }

    /// Check if this frame contains another frame entirely
    pub fn contains(&self, other: &IosFrame) -> bool {
        self.x <= other.x
            && self.y <= other.y
            && (self.x + self.width) >= (other.x + other.width)
            && (self.y + self.height) >= (other.y + other.height)
    }
}

/// Represents an iOS UI element from accessibility tree
#[derive(Debug, Clone, Deserialize, Default)]
pub struct IosElement {
    /// Accessibility label
    #[serde(default, alias = "AXLabel")]
    pub label: Option<String>,

    /// Accessibility identifier
    #[serde(default, alias = "AXUniqueId")]
    pub identifier: Option<String>,

    /// Element type (e.g., "Button", "TextField", "StaticText")
    #[serde(rename = "type", default)]
    pub element_type: Option<String>,

    /// Element frame/bounds
    #[serde(default)]
    pub frame: IosFrame,

    /// Accessibility value
    #[serde(default, alias = "AXValue")]
    pub value: Option<String>,

    /// Placeholder text (for text fields)
    #[serde(default)]
    pub placeholder: Option<String>,

    /// Whether element is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Whether element is visible
    #[serde(default = "default_true")]
    pub visible: bool,

    /// Child elements
    #[serde(default)]
    pub children: Vec<IosElement>,
}

fn default_true() -> bool {
    true
}

impl IosElement {
    /// Check if element matches the given text exactly (case-sensitive)
    pub fn matches_text_exact(&self, text: &str) -> bool {
        if let Some(label) = &self.label {
            if label == text {
                return true;
            }
        }
        if let Some(value) = &self.value {
            if value == text {
                return true;
            }
        }
        false
    }

    /// Check if element matches the given text (case-insensitive)
    pub fn matches_text(&self, text: &str) -> bool {
        // First try exact match
        if self.matches_text_exact(text) {
            return true;
        }
        // Then try case-insensitive match
        let text_lower = text.to_lowercase();
        if let Some(label) = &self.label {
            if label.to_lowercase() == text_lower || label.to_lowercase().contains(&text_lower) {
                return true;
            }
        }
        if let Some(value) = &self.value {
            if value.to_lowercase() == text_lower || value.to_lowercase().contains(&text_lower) {
                return true;
            }
        }
        false
    }

    /// Check if element matches the given text with regex
    pub fn matches_text_regex(&self, pattern: &regex::Regex) -> bool {
        if let Some(label) = &self.label {
            if pattern.is_match(label) {
                return true;
            }
        }
        if let Some(value) = &self.value {
            if pattern.is_match(value) {
                return true;
            }
        }
        false
    }

    /// Check if element matches the given accessibility ID
    pub fn matches_id(&self, id: &str) -> bool {
        self.identifier.as_ref().map_or(false, |i| i == id)
    }

    /// Check if element matches the given accessibility ID with regex
    pub fn matches_id_regex(&self, pattern: &regex::Regex) -> bool {
        self.identifier
            .as_ref()
            .map_or(false, |i| pattern.is_match(i))
    }

    /// Check if element matches the given type
    pub fn matches_type(&self, element_type: &str) -> bool {
        self.element_type.as_ref().map_or(false, |t| {
            t.eq_ignore_ascii_case(element_type) || t.ends_with(element_type)
        })
    }

    /// Check if element matches placeholder text
    pub fn matches_placeholder(&self, placeholder_text: &str) -> bool {
        self.placeholder
            .as_ref()
            .map_or(false, |p| p.contains(placeholder_text))
    }

    /// Check if element label matches text
    pub fn matches_label(&self, text: &str) -> bool {
        self.label.as_deref().unwrap_or("") == text
    }

    /// Get the center coordinates of this element
    pub fn center(&self) -> (i32, i32) {
        self.frame.center()
    }
}

/// Parse the JSON output from `idb ui describe-all`
pub fn parse_ui_hierarchy(json_output: &str) -> Result<Vec<IosElement>> {
    // idb output can be a single object or array
    // Try parsing as array first
    if let Ok(elements) = serde_json::from_str::<Vec<IosElement>>(json_output) {
        return Ok(elements);
    }

    // Try as single element
    if let Ok(element) = serde_json::from_str::<IosElement>(json_output) {
        return Ok(vec![element]);
    }

    // Try line-by-line JSON
    let mut elements = Vec::new();
    for line in json_output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(element) = serde_json::from_str::<IosElement>(line) {
            elements.push(element);
        }
    }

    if elements.is_empty() {
        anyhow::bail!("Failed to parse UI hierarchy JSON");
    }

    Ok(elements)
}

/// Flatten the element tree into a list
pub fn flatten_elements(elements: &[IosElement]) -> Vec<&IosElement> {
    let mut result = Vec::new();

    fn flatten_recursive<'a>(element: &'a IosElement, result: &mut Vec<&'a IosElement>) {
        result.push(element);
        for child in &element.children {
            flatten_recursive(child, result);
        }
    }

    for element in elements {
        flatten_recursive(element, &mut result);
    }

    result
}

/// Find elements matching text
pub fn find_by_text<'a>(
    elements: &'a [IosElement],
    text: &str,
    index: usize,
) -> Option<&'a IosElement> {
    let flat = flatten_elements(elements);
    let matches: Vec<_> = flat
        .into_iter()
        .filter(|e| e.visible && e.matches_text(text))
        .collect();
    matches.get(index).copied()
}

/// Find elements matching text regex
pub fn find_by_text_regex<'a>(
    elements: &'a [IosElement],
    pattern: &regex::Regex,
    index: usize,
) -> Option<&'a IosElement> {
    let flat = flatten_elements(elements);
    let matches: Vec<_> = flat
        .into_iter()
        .filter(|e| e.visible && e.matches_text_regex(pattern))
        .collect();
    matches.get(index).copied()
}

/// Find elements by accessibility ID
pub fn find_by_id<'a>(
    elements: &'a [IosElement],
    id: &str,
    index: usize,
) -> Option<&'a IosElement> {
    let flat = flatten_elements(elements);
    let matches: Vec<_> = flat
        .into_iter()
        .filter(|e| e.visible && e.matches_id(id))
        .collect();
    matches.get(index).copied()
}

/// Find elements by accessibility ID regex
pub fn find_by_id_regex<'a>(
    elements: &'a [IosElement],
    pattern: &regex::Regex,
    index: usize,
) -> Option<&'a IosElement> {
    let flat = flatten_elements(elements);
    let matches: Vec<_> = flat
        .into_iter()
        .filter(|e| e.visible && e.matches_id_regex(pattern))
        .collect();
    matches.get(index).copied()
}

/// Find elements by type
pub fn find_by_type<'a>(
    elements: &'a [IosElement],
    element_type: &str,
    index: usize,
) -> Option<&'a IosElement> {
    let flat = flatten_elements(elements);
    let matches: Vec<_> = flat
        .into_iter()
        .filter(|e| e.visible && e.matches_type(element_type))
        .collect();
    matches.get(index).copied()
}

/// Find elements by placeholder
pub fn find_by_placeholder<'a>(
    elements: &'a [IosElement],
    placeholder: &str,
    index: usize,
) -> Option<&'a IosElement> {
    let flat = flatten_elements(elements);
    let matches: Vec<_> = flat
        .into_iter()
        .filter(|e| e.visible && e.matches_placeholder(placeholder))
        .collect();
    matches.get(index).copied()
}

/// Find element containing a point
pub fn find_at_point<'a>(elements: &'a [IosElement], x: i32, y: i32) -> Option<&'a IosElement> {
    let flat = flatten_elements(elements);

    // Find the smallest (most specific) element containing the point
    flat.into_iter()
        .filter(|e| {
            e.visible
                && x as f64 >= e.frame.x
                && x as f64 <= e.frame.x + e.frame.width
                && y as f64 >= e.frame.y
                && y as f64 <= e.frame.y + e.frame.height
        })
        .min_by(|a, b| {
            let area_a = a.frame.width * a.frame.height;
            let area_b = b.frame.width * b.frame.height;
            area_a
                .partial_cmp(&area_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Find elements by accessibility ID (matches label or identifier)
/// This is for the `description` selector - matches accessibilityLabel or accessibilityIdentifier
pub fn find_by_accessibility_id<'a>(
    elements: &'a [IosElement],
    desc: &str,
    index: usize,
) -> Option<&'a IosElement> {
    let flat = flatten_elements(elements);
    let matches: Vec<_> = flat
        .into_iter()
        .filter(|e| {
            e.visible
                && (e
                    .label
                    .as_ref()
                    .map_or(false, |l| l == desc || l.contains(desc))
                    || e.identifier.as_ref().map_or(false, |i| i == desc))
        })
        .collect();
    matches.get(index).copied()
}

/// Find elements by accessibility ID with regex
pub fn find_by_accessibility_id_regex<'a>(
    elements: &'a [IosElement],
    pattern: &regex::Regex,
    index: usize,
) -> Option<&'a IosElement> {
    let flat = flatten_elements(elements);
    let matches: Vec<_> = flat
        .into_iter()
        .filter(|e| {
            e.visible
                && (e.label.as_ref().map_or(false, |l| pattern.is_match(l))
                    || e.identifier.as_ref().map_or(false, |i| pattern.is_match(i)))
        })
        .collect();
    matches.get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_center() {
        let element = IosElement {
            frame: IosFrame {
                x: 100.0,
                y: 200.0,
                width: 50.0,
                height: 30.0,
            },
            ..Default::default()
        };
        assert_eq!(element.center(), (125, 215));
    }

    #[test]
    fn test_matches_text() {
        let element = IosElement {
            label: Some("Login Button".to_string()),
            ..Default::default()
        };
        assert!(element.matches_text("Login"));
        assert!(element.matches_text("Button"));
        assert!(!element.matches_text("Logout"));
    }
}
