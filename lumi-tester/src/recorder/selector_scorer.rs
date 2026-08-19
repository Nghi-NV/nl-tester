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
            "id" => {
                if let Some(idx) = self.index {
                    if idx > 0 {
                        format!("- {}:\n    id: \"{}\"\n    index: {}", action, self.value, idx)
                    } else {
                        format!("- {}:\n    id: \"{}\"", action, self.value)
                    }
                } else {
                    format!("- {}:\n    id: \"{}\"", action, self.value)
                }
            }
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
            "regex" => {
                if let Some(idx) = self.index {
                    if idx > 0 {
                        format!("- {}:\n    regex: \"{}\"\n    index: {}", action, self.value, idx)
                    } else {
                        format!("- {}:\n    regex: \"{}\"", action, self.value)
                    }
                } else {
                    format!("- {}:\n    regex: \"{}\"", action, self.value)
                }
            }
            "xpath" => {
                if let Some(idx) = self.index {
                    if idx > 0 {
                        format!("- {}:\n    xpath: \"{}\"\n    index: {}", action, self.value, idx)
                    } else {
                        format!("- {}:\n    xpath: \"{}\"", action, self.value)
                    }
                } else {
                    format!("- {}:\n    xpath: \"{}\"", action, self.value)
                }
            }
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
                    let type_line = if !self.value.is_empty() && self.value != "unknown" {
                        format!("    type: \"{}\"\n", self.value)
                    } else {
                        String::new()
                    };

                    let index_line = if let Some(idx) = self.index {
                        if idx > 0 {
                            format!("    index: {}\n", idx)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    let anchor_str = match anchor.selector_type.as_str() {
                        "text" => format!("\"{}\"", anchor.value),
                        _ => format!("\n      {}: \"{}\"", anchor.selector_type, anchor.value),
                    };

                    if anchor.selector_type == "text" {
                        format!("- {}:\n{}{}    {}: {}", action, type_line, index_line, dir, anchor_str)
                    } else {
                        format!("- {}:\n{}{}    {}:{}", action, type_line, index_line, dir, anchor_str)
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

        // 2. Content Description (Mapped to 'text' as requested, only if different from text)
        if !element.content_desc.is_empty() && element.content_desc != element.text {
            let (score, reason) = self.score_content_desc(&element.content_desc);

            // Check for index if multiple elements have same content_desc
            let count = self.count_by_content_desc(&element.content_desc);
            let index = if count > 1 {
                let idx = self
                    .all_elements
                    .iter()
                    .filter(|e| e.content_desc == element.content_desc)
                    .position(|e| {
                        e.bounds.left == element.bounds.left && e.bounds.top == element.bounds.top
                    })
                    .unwrap_or(0);
                if idx > 0 {
                    Some(idx)
                } else {
                    None
                }
            } else {
                None
            };

            candidates.push(SelectorCandidate {
                selector_type: "text".to_string(),
                value: element.content_desc.clone(),
                index,
                relative_anchor: None,
                relative_direction: None,
                score,
                reason,
                is_stable: true,
            });
        }

        // Placeholder / Hint (For Input fields when text is empty)
        if element.text.is_empty() && !element.hint.is_empty() && element.hint != element.content_desc {
            let (score, reason, is_stable, _, _) = self.score_text(&element.hint);
            candidates.push(SelectorCandidate {
                selector_type: "text".to_string(),
                value: element.hint.clone(),
                index: None,
                relative_anchor: None,
                relative_direction: None,
                score: score.max(80),
                reason: format!("Placeholder: {}", reason),
                is_stable,
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
                let idx = self
                    .all_elements
                    .iter()
                    .filter(|e| e.text == element.text)
                    .position(|e| {
                        e.bounds.left == element.bounds.left && e.bounds.top == element.bounds.top
                    })
                    .unwrap_or(0);
                if idx > 0 {
                    Some(idx)
                } else {
                    None
                }
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
        let mut seen_keys = std::collections::HashSet::new();
        let mut rel_count = 0;

        for cand in candidates {
            let key = format!(
                "{}:{}:{:?}:{:?}",
                cand.selector_type, cand.value, cand.index, cand.relative_direction
            );
            if !seen_keys.insert(key) {
                continue;
            }

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
        if id == "android:id/content"
            || id == "android:id/decor_content_parent"
            || id == "android:id/navigationBarBackground"
            || id == "android:id/statusBarBackground"
            || id == "android:id/custom"
        {
            return (10, "Generic system container ID".to_string(), false);
        }

        let mut score = 85u32;
        let mut reasons: Vec<String> = Vec::new();
        let mut is_stable = true;

        for pattern in AUTO_GENERATED_PATTERNS.iter() {
            if pattern.is_match(id) {
                score = score.saturating_sub(25);
                reasons.push("may be auto-generated".to_string());
                is_stable = false;
                break;
            }
        }

        if id.len() < 50 && !id.contains("_container") && !id.contains("_wrapper") {
            score = (score + 2).min(85);
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
        let mut score = 95u32;
        if desc.len() > 100 {
            score = score.saturating_sub(10);
            return (score, "Very long description".to_string());
        }
        let count = self.count_by_content_desc(desc);
        if count > 1 {
            score = score.saturating_sub(5);
            return (score, format!("{} matches", count));
        }
        (score, "Accessibility friendly".to_string())
    }

    fn score_text(&self, text: &str) -> (u32, String, bool, bool, String) {
        let mut score = 95u32;
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
        let short_class = get_short_type(&element.class);

        // Count instances of this short_class across all elements
        let instances: Vec<&UiElement> = self
            .all_elements
            .iter()
            .filter(|e| get_short_type(&e.class) == short_class)
            .collect();

        let index = instances
            .iter()
            .position(|e| {
                e.bounds.left == element.bounds.left && e.bounds.top == element.bounds.top
            })
            .unwrap_or(0);

        let mut candidates = vec![];

        candidates.push(SelectorCandidate {
            selector_type: "type".to_string(),
            value: short_class, // Use universal standard type name
            index: if index > 0 { Some(index) } else { None },
            relative_anchor: None,
            relative_direction: None,
            score: if index > 0 { 70 } else { 75 },
            reason: if index > 0 {
                format!("Type selection (index {})", index)
            } else {
                "Type selection".to_string()
            },
            is_stable: false,
        });

        candidates
    }

    fn score_xpath(&self, element: &UiElement) -> Vec<SelectorCandidate> {
        let mut candidates = vec![];

        let short_class = get_short_type(&element.class);

        // Simple class + text
        if !element.text.is_empty() {
            let value = format!("//{}[@text='{}']", short_class, element.text);
            candidates.push(SelectorCandidate {
                selector_type: "xpath".to_string(),
                value,
                index: None,
                relative_anchor: None,
                relative_direction: None,
                score: 50,
                reason: "XPath with text".to_string(),
                is_stable: true,
            });
        }

        // Simple class + content-desc (only if distinct from text)
        if !element.content_desc.is_empty() && element.content_desc != element.text {
            let value = format!(
                "//{}[@content-desc='{}']",
                short_class, element.content_desc
            );
            candidates.push(SelectorCandidate {
                selector_type: "xpath".to_string(),
                value,
                index: None,
                relative_anchor: None,
                relative_direction: None,
                score: 50,
                reason: "XPath with content-desc".to_string(),
                is_stable: true,
            });
        }

        // Simple class + id
        if !element.resource_id.is_empty() {
            let value = format!("//{}[@id='{}']", short_class, element.resource_id);
            candidates.push(SelectorCandidate {
                selector_type: "xpath".to_string(),
                value,
                index: None,
                relative_anchor: None,
                relative_direction: None,
                score: 50,
                reason: "XPath with id".to_string(),
                is_stable: true,
            });
        }

        candidates
    }

    fn score_relative(&self, element: &UiElement) -> Vec<SelectorCandidate> {
        let mut candidates = vec![];
        let short_class = get_short_type(&element.class);

        // Find potential anchors (elements with stable ID or unique text or unique content_desc)
        for anchor in &self.all_elements {
            if anchor.bounds.left == element.bounds.left && anchor.bounds.top == element.bounds.top
            {
                continue; // Skip self
            }

            let mut anchor_selector = None;
            let mut anchor_desc = String::new();

            if !anchor.text.is_empty()
                && self.count_by_text(&anchor.text) == 1
                && anchor.text.trim().len() >= 2
            {
                anchor_selector = Some(("text", anchor.text.clone()));
                anchor_desc = format!("\"{}\"", anchor.text);
            } else if !anchor.content_desc.is_empty()
                && self.count_by_content_desc(&anchor.content_desc) == 1
                && anchor.content_desc.trim().len() >= 2
            {
                anchor_selector = Some(("text", anchor.content_desc.clone()));
                anchor_desc = format!("\"{}\"", anchor.content_desc);
            } else if !anchor.resource_id.is_empty()
                && !anchor.resource_id.starts_with("android:id/")
                && self.count_by_id(&anchor.resource_id) == 1
            {
                anchor_selector = Some(("id", anchor.resource_id.clone()));
                anchor_desc = format!("id \"{}\"", anchor.resource_id);
            }

            if anchor_selector.is_none() {
                continue;
            }
            let (sel_type, sel_val) = anchor_selector.unwrap();

            // Geometric relations
            let ab = &anchor.bounds;
            let eb = &element.bounds;
            let (ax, ay) = ab.center();
            let (ex, ey) = eb.center();

            let mut best_dir = None;
            let mut min_dist = 10000;

            // Check Below (e.g. slider below label "Brightness")
            if eb.top >= ab.bottom - 50 && (ex - ax).abs() < 500 {
                let dist = (eb.top as i32 - ab.bottom as i32).abs() as u32;
                if dist < 800 && dist < min_dist {
                    best_dir = Some(("below", dist));
                    min_dist = dist;
                }
            }
            // Check RightOf
            else if eb.left >= ab.right - 50 && (ey - ay).abs() < 300 {
                let dist = (eb.left as i32 - ab.right as i32).abs() as u32;
                if dist < 800 && dist < min_dist {
                    best_dir = Some(("rightOf", dist));
                    min_dist = dist;
                }
            }
            // Check LeftOf
            else if eb.right <= ab.left + 50 && (ey - ay).abs() < 300 {
                let dist = (ab.left as i32 - eb.right as i32).abs() as u32;
                if dist < 800 && dist < min_dist {
                    best_dir = Some(("leftOf", dist));
                    min_dist = dist;
                }
            }
            // Check Above
            else if eb.bottom <= ab.top + 50 && (ex - ax).abs() < 500 {
                let dist = (ab.top as i32 - eb.bottom as i32).abs() as u32;
                if dist < 800 && dist < min_dist {
                    best_dir = Some(("above", dist));
                    min_dist = dist;
                }
            }

            if let Some((dir, dist)) = best_dir {
                let score = 65u32.saturating_sub((dist / 15) as u32);

                // Compute relative index among all matching elements in that direction from anchor
                let mut matching_rel_elements: Vec<&UiElement> = self
                    .all_elements
                    .iter()
                    .filter(|other| {
                        if get_short_type(&other.class) != short_class {
                            return false;
                        }
                        let ot_b = &other.bounds;
                        let (ot_x, ot_y) = ot_b.center();
                        match dir {
                            "below" => ot_b.top >= ab.bottom - 50 && (ot_x - ax).abs() < 500 && (ot_b.top - ab.bottom).abs() < 800,
                            "above" => ot_b.bottom <= ab.top + 50 && (ot_x - ax).abs() < 500 && (ab.top - ot_b.bottom).abs() < 800,
                            "rightOf" => ot_b.left >= ab.right - 50 && (ot_y - ay).abs() < 300 && (ot_b.left - ab.right).abs() < 800,
                            "leftOf" => ot_b.right <= ab.left + 50 && (ot_y - ay).abs() < 300 && (ab.left - ot_b.right).abs() < 800,
                            _ => false,
                        }
                    })
                    .collect();

                // Sort by distance from anchor
                matching_rel_elements.sort_by_key(|other| {
                    let ot_b = &other.bounds;
                    match dir {
                        "below" => (ot_b.top as i32 - ab.bottom as i32).abs(),
                        "above" => (ab.top as i32 - ot_b.bottom as i32).abs(),
                        "rightOf" => (ot_b.left as i32 - ab.right as i32).abs(),
                        "leftOf" => (ab.left as i32 - ot_b.right as i32).abs(),
                        _ => 0,
                    }
                });

                let rel_index = matching_rel_elements
                    .iter()
                    .position(|e| e.bounds.left == element.bounds.left && e.bounds.top == element.bounds.top)
                    .unwrap_or(0);

                let index = if rel_index > 0 { Some(rel_index) } else { None };

                let anchor_cand = SelectorCandidate {
                    selector_type: sel_type.to_string(),
                    value: sel_val,
                    index: None,
                    relative_anchor: None,
                    relative_direction: None,
                    score: 95,
                    reason: format!("Anchor ({})", anchor_desc),
                    is_stable: true,
                };

                candidates.push(SelectorCandidate {
                    selector_type: "relative".to_string(),
                    value: short_class.clone(),
                    index,
                    relative_anchor: Some(Box::new(anchor_cand)),
                    relative_direction: Some(dir.to_string()),
                    score: if index.is_some() { score.saturating_sub(5) } else { score },
                    reason: if let Some(idx) = index {
                        format!("{} of {} (index {})", dir, anchor_desc, idx)
                    } else {
                        format!("{} of {}", dir, anchor_desc)
                    },
                    is_stable: true,
                });
            }
        }

        candidates
    }
}

/// Helper to get a standardized, universal human-friendly type name across Android, iOS, macOS, Windows, Web
fn get_short_type(class_name: &str) -> String {
    let stripped = class_name.strip_prefix("AX").unwrap_or(class_name);
    let lower = stripped.to_lowercase();
    if lower.contains("button") || lower == "btn" {
        "Button".to_string()
    } else if lower.contains("edittext")
        || lower.contains("textfield")
        || lower.contains("input")
        || lower.contains("textarea")
        || lower.contains("textbox")
        || lower.contains("securetextfield")
        || lower == "edit"
    {
        "Input".to_string()
    } else if lower.contains("textview")
        || lower.contains("label")
        || lower.contains("statictext")
        || lower.contains("heading")
        || lower == "text"
    {
        "Text".to_string()
    } else if lower.contains("checkbox") || lower.contains("check_box") {
        "CheckBox".to_string()
    } else if lower.contains("switch") || lower.contains("toggle") {
        "Switch".to_string()
    } else if lower.contains("slider") || lower.contains("seekbar") || lower.contains("progress") {
        "Slider".to_string()
    } else if lower.contains("image") || lower.contains("icon") || lower.contains("img") {
        "Image".to_string()
    } else if lower.contains("spinner")
        || lower.contains("dropdown")
        || lower.contains("combobox")
        || lower.contains("popupbutton")
        || lower.contains("select")
    {
        "ComboBox".to_string()
    } else if lower.contains("list")
        || lower.contains("recyclerview")
        || lower.contains("table")
        || lower.contains("outline")
    {
        "List".to_string()
    } else if lower.contains("row") {
        "Row".to_string()
    } else if lower.contains("cell") {
        "Cell".to_string()
    } else if lower.contains("scroll") {
        "ScrollView".to_string()
    } else if lower.contains("window") {
        "Window".to_string()
    } else if lower.contains("group") || lower.contains("layout") {
        "Group".to_string()
    } else if lower.ends_with(".view") || lower == "view" {
        "View".to_string()
    } else {
        stripped
            .split('.')
            .last()
            .unwrap_or(stripped)
            .to_string()
    }
}
