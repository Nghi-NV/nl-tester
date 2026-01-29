//! Smart Selector Scoring System
//!
//! This module implements an intelligent scoring system to choose the best
//! selector for a UI element. The scoring prioritizes stability and maintainability:
//!
//! Priority order: ID > contentDescription > text > coordinates
//!
//! # Scoring Rules
//! - **ID (resource-id)**: 100 points (most stable)
//! - **Content Description**: 90 points (accessibility friendly)
//! - **Text (exact)**: 80 points (human readable)
//! - **Text (contains)**: 60 points (less precise)
//! - **Coordinates (%)**: 20 points (fallback only)

use crate::driver::android::uiautomator::UiElement;
use regex::Regex;
use std::sync::LazyLock;

/// Represents a selector candidate with its score and metadata
#[derive(Debug, Clone)]
pub struct SelectorCandidate {
    /// The selector type name (e.g., "id", "text", "contentDesc", "point")
    pub selector_type: String,
    /// The selector value
    pub value: String,
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
            "contentDesc" => format!("- {}:\n    contentDesc: \"{}\"", action, self.value),
            "text" => format!("- {}: \"{}\"", action, self.value),
            "point" => format!("- {}:\n    point: \"{}\"", action, self.value),
            "regex" => format!("- {}:\n    regex: \"{}\"", action, self.value),
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
        // (pattern to match, regex replacement, description)
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
    /// Screen dimensions for percentage calculation
    screen_width: u32,
    screen_height: u32,
    /// All elements on screen (for uniqueness check)
    all_elements: Vec<UiElement>,
}

impl SelectorScorer {
    /// Create a new selector scorer
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

        // 1. Resource ID (highest priority)
        if !element.resource_id.is_empty() {
            let (score, reason, is_stable) = self.score_resource_id(&element.resource_id);
            candidates.push(SelectorCandidate {
                selector_type: "id".to_string(),
                value: element.resource_id.clone(),
                score,
                reason,
                is_stable,
            });
        }

        // 2. Content Description
        if !element.content_desc.is_empty() {
            let (score, reason) = self.score_content_desc(&element.content_desc);
            candidates.push(SelectorCandidate {
                selector_type: "contentDesc".to_string(),
                value: element.content_desc.clone(),
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
                    score: score.saturating_sub(10), // Regex is slightly less preferred than exact text
                    reason: format!("{} (converted to regex)", reason),
                    is_stable,
                });
            }

            candidates.push(SelectorCandidate {
                selector_type: "text".to_string(),
                value: element.text.clone(),
                score,
                reason,
                is_stable,
            });
        }

        // 4. Coordinates (fallback)
        let (x, y) = element.bounds.center();
        let x_pct = (x as f64 / self.screen_width as f64 * 100.0).round() as u32;
        let y_pct = (y as f64 / self.screen_height as f64 * 100.0).round() as u32;

        candidates.push(SelectorCandidate {
            selector_type: "point".to_string(),
            value: format!("{}%,{}%", x_pct, y_pct),
            score: 20,
            reason: "Fallback: coordinates are not recommended".to_string(),
            is_stable: false,
        });

        // Sort by score (descending)
        candidates.sort_by(|a, b| b.score.cmp(&a.score));

        candidates
    }

    /// Score a resource ID
    fn score_resource_id(&self, id: &str) -> (u32, String, bool) {
        let mut score = 100u32;
        let mut reasons: Vec<String> = Vec::new();
        let mut is_stable = true;

        // Check for auto-generated patterns
        for pattern in AUTO_GENERATED_PATTERNS.iter() {
            if pattern.is_match(id) {
                score = score.saturating_sub(30);
                reasons.push("may be auto-generated".to_string());
                is_stable = false;
                break;
            }
        }

        // Bonus for short, semantic IDs
        if id.len() < 50 && !id.contains("_container") && !id.contains("_wrapper") {
            score = score.saturating_add(5);
            reasons.push("good semantic name".to_string());
        }

        // Check uniqueness
        let count = self.count_by_id(id);
        if count > 1 {
            score = score.saturating_sub(15);
            reasons.push(format!("{} matches", count));
            is_stable = false;
        }

        let reason = if reasons.is_empty() {
            "Stable, unique ID".to_string()
        } else {
            reasons.join(", ")
        };

        (score, reason, is_stable)
    }

    /// Score a content description
    fn score_content_desc(&self, desc: &str) -> (u32, String) {
        let mut score = 90u32;

        // Penalize very long descriptions
        if desc.len() > 100 {
            score = score.saturating_sub(10);
            return (score, "Very long description".to_string());
        }

        // Check uniqueness
        let count = self.count_by_content_desc(desc);
        if count > 1 {
            score = score.saturating_sub(10);
            return (score, format!("{} matches", count));
        }

        (score, "Accessibility friendly".to_string())
    }

    /// Score text, detecting dynamic patterns
    fn score_text(&self, text: &str) -> (u32, String, bool, bool, String) {
        let mut score = 80u32;
        let mut is_stable = true;
        let mut use_regex = false;
        let mut regex_value = String::new();
        let mut reason = "Human readable text".to_string();

        // Check for dynamic patterns
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

        // Check uniqueness
        let count = self.count_by_text(text);
        if count > 1 {
            score = score.saturating_sub(20);
            is_stable = false;
            reason = format!("{} elements with same text", count);
        }

        // Penalize very short text (might be too generic)
        if text.len() <= 2 {
            score = score.saturating_sub(15);
            reason = "Very short text, may match unexpectedly".to_string();
        }

        (score, reason, is_stable, use_regex, regex_value)
    }

    /// Count elements with matching resource ID
    fn count_by_id(&self, id: &str) -> usize {
        self.all_elements
            .iter()
            .filter(|e| e.resource_id == id || e.resource_id.ends_with(&format!("/{}", id)))
            .count()
    }

    /// Count elements with matching content description
    fn count_by_content_desc(&self, desc: &str) -> usize {
        self.all_elements
            .iter()
            .filter(|e| e.content_desc == desc)
            .count()
    }

    /// Count elements with matching text
    fn count_by_text(&self, text: &str) -> usize {
        self.all_elements.iter().filter(|e| e.text == text).count()
    }

    /// Get the best selector for an element
    pub fn best_selector(&self, element: &UiElement) -> Option<SelectorCandidate> {
        self.score_element(element).into_iter().next()
    }

    /// Get the best stable selector (score >= 70)
    pub fn best_stable_selector(&self, element: &UiElement) -> Option<SelectorCandidate> {
        self.score_element(element)
            .into_iter()
            .find(|c| c.is_stable && c.score >= 70)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::android::uiautomator::Bounds;

    fn make_element(id: &str, text: &str, content_desc: &str) -> UiElement {
        UiElement {
            class: "android.widget.Button".to_string(),
            text: text.to_string(),
            resource_id: id.to_string(),
            content_desc: content_desc.to_string(),
            bounds: Bounds {
                left: 100,
                top: 200,
                right: 300,
                bottom: 250,
            },
            clickable: true,
            enabled: true,
            focusable: true,
            hint: String::new(),
            scrollable: false,
            index: "0".to_string(),
        }
    }

    #[test]
    fn test_id_is_preferred() {
        let element = make_element("com.app:id/btn_login", "Login", "Login button");
        let scorer = SelectorScorer::new(1080, 1920, vec![element.clone()]);

        let candidates = scorer.score_element(&element);
        assert_eq!(candidates[0].selector_type, "id");
        assert!(candidates[0].score >= 100);
    }

    #[test]
    fn test_dynamic_text_detection() {
        let element = make_element("", "OTP: 123456", "");
        let scorer = SelectorScorer::new(1080, 1920, vec![element.clone()]);

        let candidates = scorer.score_element(&element);

        // Should have both regex and text options
        let regex_candidate = candidates.iter().find(|c| c.selector_type == "regex");
        assert!(regex_candidate.is_some());
    }

    #[test]
    fn test_auto_generated_id_penalty() {
        let element = make_element("com.app:id/view_1234567890", "", "");
        let scorer = SelectorScorer::new(1080, 1920, vec![element.clone()]);

        let candidates = scorer.score_element(&element);
        let id_candidate = candidates.iter().find(|c| c.selector_type == "id").unwrap();

        // Should have reduced score due to auto-generated pattern
        assert!(id_candidate.score < 100);
    }

    #[test]
    fn test_fallback_to_coords() {
        let element = make_element("", "", "");
        let scorer = SelectorScorer::new(1080, 1920, vec![element.clone()]);

        let candidates = scorer.score_element(&element);

        // Only option should be point
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].selector_type, "point");
    }
}
