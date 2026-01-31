//! Smart Selector Scoring System
//!
//! This module implements an intelligent scoring system to choose the best
//! selector for a UI element. The scoring prioritizes stability and maintainability:
//!
//! Priority order: ID > contentDescription > text > relative > xpath > coordinates
//!
//! # Scoring Rules
//! - **ID (resource-id)**: 100 points
//! - **Content Description**: 90 points
//! - **Text (exact)**: 80 points
//! - **Relative (RightOf/Below)**: 75 points
//! - **XPath**: 50 points
//! - **Type + Index**: 40 points
//! - **Coordinates (%)**: 20 points

use crate::driver::android::uiautomator::{Bounds, UiElement};
use regex::Regex;
use std::sync::LazyLock;

/// Represents a selector candidate with its score and metadata
#[derive(Debug, Clone)]
pub struct SelectorCandidate {
    /// The selector type name (e.g., "id", "text", "xpath", "relative")
    pub selector_type: String,
    /// The primary value (or specific structure for relative)
    pub value: String,
    /// Additional metadata (e.g., index for type, anchor for relative)
    pub index: Option<usize>,
    pub relative_anchor: Option<Box<SelectorCandidate>>,
    pub relative_direction: Option<String>,
    /// Score (0-100), higher is better
    pub score: u32,
    /// Human-readable explanation
    pub reason: String,
    /// Whether this selector is considered stable
    pub is_stable: bool,
}

impl SelectorCandidate {
    /// Convert to YAML representation
    pub fn to_yaml(&self, action: &str) -> String {
        match self.selector_type.as_str() {
            "id" => format!("- {}:\n    id: \"{}\"", action, self.value),
            // Both "text" and "contentDesc" (mapped to text) use explicit text key now
            "text" | "contentDesc" => {
                if let Some(idx) = self.index {
                    if idx > 0 {
                        format!(
                            "- {}:\n    text: \"{}\"\n    index: {}",
                            action, self.value, idx
                        )
                    } else {
                        format!("- {}:\n    text: \"{}\"", action, self.value)
                    }
                } else {
                    format!("- {}:\n    text: \"{}\"", action, self.value)
                }
            }
            "point" => format!("- {}:\n    point: \"{}\"", action, self.value),
            "regex" => format!("- {}:\n    regex: \"{}\"", action, self.value),
            "xpath" => format!("- {}:\n    xpath: \"{}\"", action, self.value),
            "type" => {
                // Value is already short name ("Button")
                if let Some(idx) = self.index {
                    if idx > 0 {
                        format!(
                            "- {}:\n    type: \"{}\"\n    index: {}",
                            action, self.value, idx
                        )
                    } else {
                        format!("- {}:\n    type: \"{}\"", action, self.value)
                    }
                } else {
                    format!("- {}:\n    type: \"{}\"", action, self.value)
                }
            }
            "relative" => {
                if let (Some(anchor), Some(dir)) = (&self.relative_anchor, &self.relative_direction)
                {
                    // Short representation for anchor if possible
                    let anchor_str = match anchor.selector_type.as_str() {
                        "text" => format!("\"{}\"", anchor.value),
                        _ => format!("\n      {}: \"{}\"", anchor.selector_type, anchor.value),
                    };

                    if anchor.selector_type == "text" {
                        format!("- {}:\n    {}: {}", action, dir, anchor_str)
                    } else {
                        format!("- {}:\n    {}:{}", action, dir, anchor_str)
                    }
                } else {
                    format!("- {}: \"unknown relative\"", action)
                }
            }
            _ => format!("- {}: \"{}\"", action, self.value),
        }
    }

    /// Get a short representation for comments
    pub fn short_repr(&self) -> String {
        match self.selector_type.as_str() {
            "id" => format!("id=\"{}\"", self.value),
            "contentDesc" => format!("contentDesc=\"{}\"", self.value),
            "text" => format!("text=\"{}\"", self.value),
            "point" => format!("point=\"{}\"", self.value),
            "regex" => format!("regex=\"{}\"", self.value),
            "xpath" => format!("xpath=\"{}\"", self.value),
            "relative" => {
                if let Some(dir) = &self.relative_direction {
                    format!("relative={}", dir)
                } else {
                    "relative".to_string()
                }
            }
            _ => self.value.clone(),
        }
    }
}

/// Patterns that indicate auto-generated resource IDs (less stable)
static AUTO_GENERATED_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}").unwrap(), // UUID pattern
        Regex::new(r"_\d{10,}").unwrap(),                // Timestamp suffix
        Regex::new(r"[A-Za-z]+\d{5,}").unwrap(),         // Random number suffix
        Regex::new(r"^generated_").unwrap(),             // Explicit generated prefix
    ]
});

/// Dynamic text patterns that should be converted to regex
static DYNAMIC_PATTERNS: LazyLock<Vec<(Regex, &'static str, &'static str)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"^\d{4,6}$").unwrap(), r"\\d{4,6}", "OTP code"),
        (
            Regex::new(r"0[0-9]{9,10}").unwrap(),
            r"0\\d{9,10}",
            "Phone number",
        ),
        (
            Regex::new(r"\d{1,3}(,\d{3})+").unwrap(),
            r"\\d{1,3}(,\\d{3})+",
            "Formatted number",
        ),
        (
            Regex::new(r"\d{1,2}/\d{1,2}/\d{4}").unwrap(),
            r"\\d{1,2}/\\d{1,2}/\\d{4}",
            "Date",
        ),
        (
            Regex::new(r"\d{1,2}:\d{2}(:\d{2})?").unwrap(),
            r"\\d{1,2}:\\d{2}(:\\d{2})?",
            "Time",
        ),
    ]
});

/// Smart selector scorer
pub struct SelectorScorer {
    screen_width: u32,
    screen_height: u32,
    all_elements: Vec<UiElement>,
}

impl SelectorScorer {
    pub fn new(screen_width: u32, screen_height: u32, all_elements: Vec<UiElement>) -> Self {
        Self {
            screen_width,
            screen_height,
            all_elements,
        }
    }

    /// Score all possible selectors for an element and return them sorted by score
    pub fn score_element(&self, element: &UiElement) -> Vec<SelectorCandidate> {
        let mut candidates = Vec::new();

        // 1. Resource ID
        if !element.resource_id.is_empty() {
            let (score, reason, is_stable) = self.score_resource_id(&element.resource_id);
            candidates.push(SelectorCandidate {
                selector_type: "id".to_string(),
                value: element.resource_id.clone(),
                index: None,
                relative_anchor: None,
                relative_direction: None,
                score,
                reason,
                is_stable,
            });
        }

        // 2. Content Description (Mapped to 'text' as requested)
        if !element.content_desc.is_empty() {
            let (score, reason) = self.score_content_desc(&element.content_desc);

            // Check for index if multiple elements have same content_desc
            let count = self.count_by_content_desc(&element.content_desc);
            let index = if count > 1 {
                self.all_elements
                    .iter()
                    .filter(|e| e.content_desc == element.content_desc)
                    .position(|e| {
                        e.bounds.left == element.bounds.left && e.bounds.top == element.bounds.top
                    })
            } else {
                None
            };

            candidates.push(SelectorCandidate {
                selector_type: "text".to_string(), // CHANGED: Internal type is now text as requested
                value: element.content_desc.clone(),
                index,
                relative_anchor: None,
                relative_direction: None,
                score,
                reason,
                is_stable: true,
            });
        }

        // 3. Text
        if !element.text.is_empty() {
            let (score, reason, is_stable, use_regex, regex_value) = self.score_text(&element.text);

            if use_regex {
                candidates.push(SelectorCandidate {
                    selector_type: "regex".to_string(),
                    value: regex_value,
                    index: None,
                    relative_anchor: None,
                    relative_direction: None,
                    score: score.saturating_sub(10),
                    reason: format!("{} (converted to regex)", reason),
                    is_stable,
                });
            }

            // Check if text is unique or needs index
            let text_count = self.count_by_text(&element.text);
            let index = if text_count > 1 {
                // Calculate which instance this is
                // Simplified: assume we find it by position (top-left first)
                let idx = self
                    .all_elements
                    .iter()
                    .filter(|e| e.text == element.text)
                    .position(|e| {
                        e.bounds.left == element.bounds.left && e.bounds.top == element.bounds.top
                    })
                    .unwrap_or(0);
                Some(idx)
            } else {
                None
            };

            let final_score = if index.is_some() {
                score.saturating_sub(10)
            } else {
                score
            };

            candidates.push(SelectorCandidate {
                selector_type: "text".to_string(),
                value: element.text.clone(),
                index,
                relative_anchor: None,
                relative_direction: None,
                score: final_score,
                reason: if index.is_some() {
                    format!("{}, index {}", reason, index.unwrap())
                } else {
                    reason
                },
                is_stable,
            });
        }

        // 4. Relative Selectors (New!)
        let relative_selectors = self.score_relative(element);
        candidates.extend(relative_selectors);

        // 5. XPath
        let xpath_selectors = self.score_xpath(element);
        candidates.extend(xpath_selectors);

        // 6. Type + Index
        let type_selectors = self.score_type(element);
        candidates.extend(type_selectors);

        // 7. Coordinates (fallback)
        let (x, y) = element.bounds.center();
        let x_pct = (x as f64 / self.screen_width as f64 * 100.0).round() as u32;
        let y_pct = (y as f64 / self.screen_height as f64 * 100.0).round() as u32;

        // % format
        candidates.push(SelectorCandidate {
            selector_type: "point".to_string(),
            value: format!("{}%,{}%", x_pct, y_pct),
            index: None,
            relative_anchor: None,
            relative_direction: None,
            score: 20,
            reason: "Coordinates (percentage)".to_string(),
            is_stable: false,
        });

        // x,y format
        candidates.push(SelectorCandidate {
            selector_type: "point".to_string(),
            value: format!("{},{}", x, y),
            index: None,
            relative_anchor: None,
            relative_direction: None,
            score: 15,
            reason: "Coordinates (absolute pixels)".to_string(),
            is_stable: false,
        });

        // Sort by score (descending)
        candidates.sort_by(|a, b| b.score.cmp(&a.score));

        // Limit relative candidates
        // If we have no ID and no Text, keep more relative candidates
        // NOTE: "Type" is NOT considered strong, so we allow relative selectors to show up
        let has_strong_selector = candidates
            .iter()
            .any(|c| (c.selector_type == "id" || c.selector_type == "text") && c.is_stable);

        // Boost limits: if no strong selector, take up to 4 relative ones
        let max_rel = if has_strong_selector { 1 } else { 4 };
        let mut final_candidates = Vec::new();
        let mut rel_count = 0;

        for cand in candidates {
            if cand.selector_type == "relative" {
                if rel_count < max_rel {
                    final_candidates.push(cand);
                    rel_count += 1;
                }
            } else {
                final_candidates.push(cand);
            }
        }

        final_candidates
    }

    // ... (Existing score helper functions) ...
    fn score_resource_id(&self, id: &str) -> (u32, String, bool) {
        let mut score = 100u32;
        let mut reasons: Vec<String> = Vec::new();
        let mut is_stable = true;

        for pattern in AUTO_GENERATED_PATTERNS.iter() {
            if pattern.is_match(id) {
                score = score.saturating_sub(30);
                reasons.push("may be auto-generated".to_string());
                is_stable = false;
                break;
            }
        }

        if id.len() < 50 && !id.contains("_container") && !id.contains("_wrapper") {
            score = score.saturating_add(5);
            reasons.push("good semantic name".to_string());
        }

        let count = self.count_by_id(id);
        if count > 1 {
            score = score.saturating_sub(15);
            reasons.push(format!("{} matches", count));
            is_stable = false;
        }

        (
            score,
            if reasons.is_empty() {
                "Stable, unique ID".to_string()
            } else {
                reasons.join(", ")
            },
            is_stable,
        )
    }

    fn score_content_desc(&self, desc: &str) -> (u32, String) {
        let mut score = 90u32;
        if desc.len() > 100 {
            score = score.saturating_sub(10);
            return (score, "Very long description".to_string());
        }
        let count = self.count_by_content_desc(desc);
        if count > 1 {
            score = score.saturating_sub(10);
            return (score, format!("{} matches", count));
        }
        (score, "Accessibility friendly".to_string())
    }

    fn score_text(&self, text: &str) -> (u32, String, bool, bool, String) {
        let mut score = 80u32;
        let mut is_stable = true;
        let mut use_regex = false;
        let mut regex_value = String::new();
        let mut reason = "Human readable text".to_string();

        for (pattern, replacement, desc) in DYNAMIC_PATTERNS.iter() {
            if pattern.is_match(text) {
                use_regex = true;
                regex_value = pattern.replace_all(text, *replacement).to_string();
                is_stable = false;
                reason = format!("Dynamic text detected: {}", desc);
                score = score.saturating_sub(20);
                break;
            }
        }

        let count = self.count_by_text(text);
        if count > 1 {
            // Uniqueness is handled by index
            is_stable = false;
            reason = format!("{} elements with same text", count);
        }

        if text.len() <= 2 {
            score = score.saturating_sub(15);
            reason = "Very short text, may match unexpectedly".to_string();
        }

        (score, reason, is_stable, use_regex, regex_value)
    }

    fn count_by_id(&self, id: &str) -> usize {
        self.all_elements
            .iter()
            .filter(|e| e.resource_id == id || e.resource_id.ends_with(&format!("/{}", id)))
            .count()
    }

    fn count_by_content_desc(&self, desc: &str) -> usize {
        self.all_elements
            .iter()
            .filter(|e| e.content_desc == desc)
            .count()
    }

    fn count_by_text(&self, text: &str) -> usize {
        self.all_elements.iter().filter(|e| e.text == text).count()
    }

    // --- NEW STRATEGIES ---

    fn score_type(&self, element: &UiElement) -> Vec<SelectorCandidate> {
        // e.g. type: android.widget.Button
        let short_type = element.class.split('.').last().unwrap_or(&element.class);

        // Count instances of this type
        let instances: Vec<&UiElement> = self
            .all_elements
            .iter()
            .filter(|e| e.class == element.class || e.class.ends_with(short_type))
            .collect();

        let index = instances
            .iter()
            .position(|e| {
                e.bounds.left == element.bounds.left && e.bounds.top == element.bounds.top
            })
            .unwrap_or(0);

        let mut candidates = vec![];

        // Short Value
        let short_class = get_short_type(&element.class);

        candidates.push(SelectorCandidate {
            selector_type: "type".to_string(),
            value: short_class, // Use short name directly as value
            index: Some(index),
            relative_anchor: None,
            relative_direction: None,
            score: 40,
            reason: format!("Type selection (index {})", index),
            is_stable: false,
        });

        candidates
    }

    fn score_xpath(&self, element: &UiElement) -> Vec<SelectorCandidate> {
        let mut candidates = vec![];

        // Simple class + text
        if !element.text.is_empty() {
            let value =
                format!("//{}[\n@text='{}']", element.class, element.text).replace("\n", "");
            // Note: using direct @ works, but just to be safe I'll use simple string
            let value = format!("//{}[@text='{}']", element.class, element.text);
            candidates.push(SelectorCandidate {
                selector_type: "xpath".to_string(),
                value,
                index: None,
                relative_anchor: None,
                relative_direction: None,
                score: 55,
                reason: "XPath with text".to_string(),
                is_stable: true,
            });
        }

        // Simple class + content-desc
        if !element.content_desc.is_empty() {
            let value = format!(
                "//{}[@content-desc='{}']",
                element.class, element.content_desc
            );
            candidates.push(SelectorCandidate {
                selector_type: "xpath".to_string(),
                value,
                index: None,
                relative_anchor: None,
                relative_direction: None,
                score: 55,
                reason: "XPath with content-desc".to_string(),
                is_stable: true,
            });
        }

        candidates
    }

    fn score_relative(&self, element: &UiElement) -> Vec<SelectorCandidate> {
        let mut candidates = vec![];

        // Find potential anchors
        // Valid anchor:
        // 1. Must use ID or Text (stable)
        // 2. Must be unique
        // 3. Must not be the element itself
        // 4. Must be close to the element

        for anchor in &self.all_elements {
            if anchor.resource_id == element.resource_id
                && anchor.text == element.text
                && anchor.bounds.left == element.bounds.left
            {
                continue; // Skip self
            }

            // Check anchor stability
            let mut anchor_selector = None;
            if !anchor.text.is_empty()
                && self.count_by_text(&anchor.text) == 1
                && anchor.text.len() > 3
            {
                anchor_selector = Some(("text", &anchor.text));
            } else if !anchor.resource_id.is_empty() && self.count_by_id(&anchor.resource_id) == 1 {
                anchor_selector = Some(("id", &anchor.resource_id));
            }

            if anchor_selector.is_none() {
                continue;
            }
            let (sel_type, sel_val) = anchor_selector.unwrap();

            // Geometric relations (using Inclusive check fixed in previous task)
            let ab = &anchor.bounds;
            let eb = &element.bounds;
            let (ax, ay) = ab.center();
            let (ex, ey) = eb.center();

            let mut best_dir = None;
            let mut min_dist = 10000;

            // Check RightOf
            // Condition: Relaxed overlap (eb.left can be slightly less than ab.right)
            // Allow 50px overlap
            if eb.left >= ab.right - 50 && (ey - ay).abs() < 200 {
                let dist = (eb.left as i32 - ab.right as i32).abs() as u32;
                if dist < 600 {
                    best_dir = Some(("rightOf", dist));
                    min_dist = dist;
                }
            }
            // Check Below
            else if eb.top >= ab.bottom - 50 && (ex - ax).abs() < 200 {
                let dist = (eb.top as i32 - ab.bottom as i32).abs() as u32;
                if dist < 600 {
                    if best_dir.is_none() || dist < min_dist {
                        best_dir = Some(("below", dist));
                        min_dist = dist;
                    }
                }
            }
            // Check LeftOf
            else if eb.right <= ab.left + 50 && (ey - ay).abs() < 200 {
                let dist = (ab.left as i32 - eb.right as i32).abs() as u32;
                if dist < 600 {
                    if best_dir.is_none() || dist < min_dist {
                        best_dir = Some(("leftOf", dist));
                        min_dist = dist;
                    }
                }
            }
            // Check Above
            else if eb.bottom <= ab.top + 50 && (ex - ax).abs() < 200 {
                let dist = (ab.top as i32 - eb.bottom as i32).abs() as u32;
                if dist < 600 {
                    if best_dir.is_none() || dist < min_dist {
                        best_dir = Some(("above", dist));
                        min_dist = dist;
                    }
                }
            }

            if let Some((dir, dist)) = best_dir {
                let score = 75u32.saturating_sub((dist / 10) as u32);

                let anchor_cand = SelectorCandidate {
                    selector_type: sel_type.to_string(),
                    value: sel_val.to_string(),
                    index: None,
                    relative_anchor: None,
                    relative_direction: None,
                    score: 80,
                    reason: "Anchor".to_string(),
                    is_stable: true,
                };

                candidates.push(SelectorCandidate {
                    selector_type: "relative".to_string(),
                    value: format!("{}: {}", dir, sel_val), // e.g. rightOf: "Submit"
                    index: None,
                    relative_anchor: Some(Box::new(anchor_cand)),
                    relative_direction: Some(dir.to_string()),
                    score,
                    reason: format!("Relative to stable anchor {} (dist={})", sel_val, dist),
                    is_stable: true,
                });
            }
        }

        candidates
    }
}

/// Helper to get a short, human-friendly type name
fn get_short_type(class_name: &str) -> String {
    let lower = class_name.to_lowercase();
    if lower.contains("button") {
        "Button".to_string()
    } else if lower.contains("edittext") || lower.contains("input") {
        "Input".to_string()
    } else if lower.contains("textview") || lower.contains("label") {
        "Text".to_string()
    } else if lower.contains("image") || lower.contains("icon") {
        "Image".to_string()
    } else if lower.contains("checkbox") {
        "Checkbox".to_string()
    } else if lower.contains("switch") {
        "Switch".to_string()
    } else if lower.contains("spinner") || lower.contains("dropdown") {
        "Dropdown".to_string()
    } else if lower.contains("list") || lower.contains("recyclerview") {
        "List".to_string()
    } else {
        class_name
            .split('.')
            .last()
            .unwrap_or(class_name)
            .to_string()
    }
}
