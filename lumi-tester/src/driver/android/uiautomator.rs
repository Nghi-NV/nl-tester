use crate::driver::traits::RelativeDirection;
use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;

/// Decode common HTML entities in a string
/// Handles: &amp; &lt; &gt; &quot; &apos; &#NNN; (decimal) &#xHHH; (hex)
fn decode_html_entities(s: &str) -> String {
    let mut result = s.to_string();

    // Named entities
    result = result.replace("&amp;", "&");
    result = result.replace("&lt;", "<");
    result = result.replace("&gt;", ">");
    result = result.replace("&quot;", "\"");
    result = result.replace("&apos;", "'");
    result = result.replace("&nbsp;", " ");

    // Numeric entities (decimal): &#NNN;
    let decimal_re = Regex::new(r"&#(\d+);").unwrap();
    result = decimal_re
        .replace_all(&result, |caps: &regex::Captures| {
            if let Ok(code) = caps[1].parse::<u32>() {
                if let Some(c) = char::from_u32(code) {
                    return c.to_string();
                }
            }
            caps[0].to_string()
        })
        .to_string();

    // Numeric entities (hex): &#xHHH;
    let hex_re = Regex::new(r"&#x([0-9A-Fa-f]+);").unwrap();
    result = hex_re
        .replace_all(&result, |caps: &regex::Captures| {
            if let Ok(code) = u32::from_str_radix(&caps[1], 16) {
                if let Some(c) = char::from_u32(code) {
                    return c.to_string();
                }
            }
            caps[0].to_string()
        })
        .to_string();

    result
}

/// Represents a UI element from the view hierarchy
#[derive(Debug, Clone)]
pub struct UiElement {
    pub class: String,
    pub text: String,
    pub resource_id: String,
    pub content_desc: String,
    pub bounds: Bounds,
    pub clickable: bool,
    pub enabled: bool,
    pub focusable: bool,
    pub hint: String,
    pub scrollable: bool,
    pub index: String,
}

#[derive(Debug, Clone, Default)]
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Bounds {
    /// Get the center point of the bounds
    pub fn center(&self) -> (i32, i32) {
        let x = (self.left + self.right) / 2;
        let y = (self.top + self.bottom) / 2;
        (x, y)
    }

    /// Parse bounds from string like "[0,0][1080,1920]"
    pub fn from_string(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split("][").collect();
        if parts.len() != 2 {
            return None;
        }

        let left_top = parts[0].trim_start_matches('[');
        let right_bottom = parts[1].trim_end_matches(']');

        let lt: Vec<i32> = left_top.split(',').filter_map(|s| s.parse().ok()).collect();
        let rb: Vec<i32> = right_bottom
            .split(',')
            .filter_map(|s| s.parse().ok())
            .collect();

        if lt.len() == 2 && rb.len() == 2 {
            Some(Bounds {
                left: lt[0],
                top: lt[1],
                right: rb[0],
                bottom: rb[1],
            })
        } else {
            None
        }
    }
}

/// Parse UI hierarchy XML from uiautomator dump
pub fn parse_hierarchy(xml: &str) -> Result<Vec<UiElement>> {
    let mut elements = Vec::new();
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                if e.name().as_ref() == b"node" {
                    let mut element = UiElement {
                        class: String::new(),
                        text: String::new(),
                        resource_id: String::new(),
                        content_desc: String::new(),
                        bounds: Bounds::default(),
                        clickable: false,
                        enabled: true,
                        focusable: false,
                        hint: String::new(),
                        scrollable: false,
                        index: String::new(),
                    };

                    for attr in e.attributes().filter_map(|a| a.ok()) {
                        let key = String::from_utf8_lossy(attr.key.as_ref());
                        let value = String::from_utf8_lossy(&attr.value);

                        match key.as_ref() {
                            "class" => element.class = value.to_string(),
                            "text" => element.text = decode_html_entities(&value),
                            "resource-id" => element.resource_id = value.to_string(),
                            "content-desc" => element.content_desc = decode_html_entities(&value),
                            "bounds" => {
                                if let Some(b) = Bounds::from_string(&value) {
                                    element.bounds = b;
                                }
                            }
                            "clickable" => element.clickable = value == "true",
                            "enabled" => element.enabled = value == "true",
                            "focusable" => element.focusable = value == "true",
                            "hint" => element.hint = decode_html_entities(&value),
                            "scrollable" => element.scrollable = value == "true",
                            "index" => element.index = value.to_string(),
                            _ => {}
                        }
                    }

                    elements.push(element);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                eprintln!("XML parse error: {:?}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(elements)
}

/// Find element by text
pub fn find_by_text<'a>(elements: &'a [UiElement], text: &str) -> Option<&'a UiElement> {
    elements
        .iter()
        .find(|e| e.text == text || e.content_desc == text)
}

/// Find element by resource ID
pub fn find_by_id<'a>(elements: &'a [UiElement], id: &str) -> Option<&'a UiElement> {
    elements
        .iter()
        .find(|e| e.resource_id == id || e.resource_id.ends_with(&format!("/{}", id)))
}

/// Find element by partial text match
pub fn find_by_text_contains<'a>(elements: &'a [UiElement], text: &str) -> Option<&'a UiElement> {
    elements
        .iter()
        .find(|e| e.text.contains(text) || e.content_desc.contains(text))
}

/// Find nth element by partial text match
/// Normalize text: replace NBSP with space, trim whitespace
fn normalize_text(s: &str) -> String {
    s.replace('\u{00A0}', " ").trim().to_string()
}

/// Find nth element containing text
pub fn find_nth_by_text_contains<'a>(
    elements: &'a [UiElement],
    text: &str,
    index: u32,
) -> Option<&'a UiElement> {
    let text_norm = normalize_text(text);
    elements
        .iter()
        .filter(|e| {
            normalize_text(&e.text).contains(&text_norm)
                || normalize_text(&e.content_desc).contains(&text_norm)
        })
        .nth(index as usize)
}

/// Find all elements by class type (e.g., "EditText", "Button")
pub fn find_all_by_type<'a>(elements: &'a [UiElement], class_type: &str) -> Vec<&'a UiElement> {
    elements
        .iter()
        .filter(|e| e.class.contains(class_type) || e.class.ends_with(&format!(".{}", class_type)))
        .collect()
}

/// Find element by class type and index (0-based)
pub fn find_by_type_index<'a>(
    elements: &'a [UiElement],
    class_type: &str,
    index: u32,
) -> Option<&'a UiElement> {
    find_all_by_type(elements, class_type)
        .get(index as usize)
        .copied()
}

/// Find nth element matching text (with case-insensitive fallback)
pub fn find_nth_by_text<'a>(
    elements: &'a [UiElement],
    text: &str,
    index: u32,
) -> Option<&'a UiElement> {
    let text_norm = normalize_text(text);

    // First try exact match (normalized)
    let exact_match = elements
        .iter()
        .filter(|e| {
            normalize_text(&e.text) == text_norm
                || normalize_text(&e.content_desc) == text_norm
                || normalize_text(&e.hint) == text_norm
        })
        .nth(index as usize);

    if exact_match.is_some() {
        return exact_match;
    }

    // Fallback to case-insensitive match (normalized)
    let text_lower = text_norm.to_lowercase();
    elements
        .iter()
        .filter(|e| {
            normalize_text(&e.text).to_lowercase() == text_lower
                || normalize_text(&e.content_desc).to_lowercase() == text_lower
                || normalize_text(&e.hint).to_lowercase() == text_lower
        })
        .nth(index as usize)
}

/// Find nth element matching text (exact match only, no fallback)
pub fn find_nth_by_text_exact<'a>(
    elements: &'a [UiElement],
    text: &str,
    index: u32,
) -> Option<&'a UiElement> {
    let text_norm = normalize_text(text);
    elements
        .iter()
        .filter(|e| {
            normalize_text(&e.text) == text_norm
                || normalize_text(&e.content_desc) == text_norm
                || normalize_text(&e.hint) == text_norm
        })
        .nth(index as usize)
}

/// Find element matching regex pattern on text or content description
pub fn find_by_regex<'a>(elements: &'a [UiElement], pattern: &str) -> Option<&'a UiElement> {
    match Regex::new(pattern) {
        Ok(re) => elements
            .iter()
            .find(|e| re.is_match(&e.text) || re.is_match(&e.content_desc) || re.is_match(&e.hint)),
        Err(e) => {
            eprintln!("Invalid regex '{}': {}", pattern, e);
            None
        }
    }
}

pub fn find_all_by_text<'a>(elements: &'a [UiElement], text: &str) -> Vec<&'a UiElement> {
    elements
        .iter()
        .filter(|e| e.text == text || e.content_desc == text || e.hint == text)
        .collect()
}

pub fn find_all_by_id<'a>(elements: &'a [UiElement], id: &str) -> Vec<&'a UiElement> {
    elements
        .iter()
        .filter(|e| e.resource_id == id || e.resource_id.ends_with(&format!("/{}", id)))
        .collect()
}

pub fn find_all_by_regex<'a>(elements: &'a [UiElement], pattern: &str) -> Vec<&'a UiElement> {
    match Regex::new(pattern) {
        Ok(re) => elements
            .iter()
            .filter(|e| {
                re.is_match(&e.text) || re.is_match(&e.content_desc) || re.is_match(&e.hint)
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Find nth element by resource ID
pub fn find_nth_by_id<'a>(
    elements: &'a [UiElement],
    id: &str,
    index: u32,
) -> Option<&'a UiElement> {
    elements
        .iter()
        .filter(|e| e.resource_id == id || e.resource_id.ends_with(&format!("/{}", id)))
        .nth(index as usize)
}

/// Find nth element matching regex pattern
pub fn find_nth_by_regex<'a>(
    elements: &'a [UiElement],
    pattern: &str,
    index: u32,
) -> Option<&'a UiElement> {
    match Regex::new(pattern) {
        Ok(re) => elements
            .iter()
            .filter(|e| {
                re.is_match(&e.text) || re.is_match(&e.content_desc) || re.is_match(&e.hint)
            })
            .nth(index as usize),
        Err(e) => {
            eprintln!("Invalid regex '{}': {}", pattern, e);
            None
        }
    }
}

pub fn find_relative<'a>(
    candidates: Vec<&'a UiElement>,
    anchor: &UiElement,
    direction: RelativeDirection,
    max_dist: Option<u32>,
) -> Vec<&'a UiElement> {
    // Use i32::MAX as default limit to avoid overflow when casting u32::MAX to i32
    let limit = max_dist.map(|d| d as i32).unwrap_or(i32::MAX);

    let mut scored_candidates: Vec<(&UiElement, bool, f64, f64)> = candidates
        .into_iter()
        .filter_map(|candidate| {
            // Filter out large container elements (>80% of screen area)
            // Screen size assumption: 1080x2340 (common Android), area = 2,527,200
            // Using simpler heuristic: width covers >80% AND height covers >50%
            let screen_width = 1080;
            let screen_height = 2340;
            let candidate_width = candidate.bounds.right - candidate.bounds.left;
            let candidate_height = candidate.bounds.bottom - candidate.bounds.top;

            let width_ratio = candidate_width as f64 / screen_width as f64;
            let height_ratio = candidate_height as f64 / screen_height as f64;

            // Skip if element:
            // 1. Covers >80% width AND >50% height (large container)
            // 2. OR covers >95% width (full-width container like HorizontalScrollView)
            // 3. OR covers >80% height (full-height container)
            if (width_ratio > 0.8 && height_ratio > 0.5) || width_ratio > 0.95 || height_ratio > 0.8
            {
                return None;
            }

            // Direction check (inclusive for adjacent elements)
            let is_valid = match direction {
                RelativeDirection::RightOf => {
                    candidate.bounds.left >= anchor.bounds.right
                        || (anchor.bounds.contains(&candidate.bounds)
                            && candidate.bounds.left >= anchor.bounds.center().0)
                }
                RelativeDirection::LeftOf => {
                    candidate.bounds.right <= anchor.bounds.left
                        || (anchor.bounds.contains(&candidate.bounds)
                            && candidate.bounds.right <= anchor.bounds.center().0)
                }
                RelativeDirection::Below => {
                    candidate.bounds.top >= anchor.bounds.bottom
                        || (anchor.bounds.contains(&candidate.bounds)
                            && candidate.bounds.top >= anchor.bounds.center().1)
                }
                RelativeDirection::Above => {
                    candidate.bounds.bottom <= anchor.bounds.top
                        || (anchor.bounds.contains(&candidate.bounds)
                            && candidate.bounds.bottom <= anchor.bounds.center().1)
                }
                RelativeDirection::Near => true,
            };

            if !is_valid {
                return None;
            }

            // Calculate bounds-based distance (edge-to-edge gap)
            let edge_dist = match direction {
                RelativeDirection::RightOf => candidate.bounds.left - anchor.bounds.right,
                RelativeDirection::LeftOf => anchor.bounds.left - candidate.bounds.right,
                RelativeDirection::Below => candidate.bounds.top - anchor.bounds.bottom,
                RelativeDirection::Above => anchor.bounds.top - candidate.bounds.bottom,
                RelativeDirection::Near => {
                    // For Near, use center-to-center distance
                    let (ax, ay) = anchor.bounds.center();
                    let (cx, cy) = candidate.bounds.center();
                    (((cx - ax).pow(2u32) + (cy - ay).pow(2u32)) as f64).sqrt() as i32
                }
            };

            if edge_dist > limit {
                return None;
            }

            // Calculate horizontal/vertical overlap (bounds-based)
            let overlap = match direction {
                RelativeDirection::RightOf | RelativeDirection::LeftOf => {
                    // Y-axis overlap for left/right
                    std::cmp::max(
                        0,
                        std::cmp::min(candidate.bounds.bottom, anchor.bounds.bottom)
                            - std::cmp::max(candidate.bounds.top, anchor.bounds.top),
                    )
                }
                RelativeDirection::Below | RelativeDirection::Above => {
                    // X-axis overlap for above/below
                    std::cmp::max(
                        0,
                        std::cmp::min(candidate.bounds.right, anchor.bounds.right)
                            - std::cmp::max(candidate.bounds.left, anchor.bounds.left),
                    )
                }
                RelativeDirection::Near => 0,
            };

            // Calculate anchor size on the alignment axis
            let anchor_size = match direction {
                RelativeDirection::RightOf | RelativeDirection::LeftOf => {
                    anchor.bounds.bottom - anchor.bounds.top // height
                }
                RelativeDirection::Below | RelativeDirection::Above => {
                    anchor.bounds.right - anchor.bounds.left // width
                }
                RelativeDirection::Near => 1,
            };

            // alignment_factor: 0.0 to 1.0 (how well aligned with anchor)
            let alignment_factor = if anchor_size > 0 {
                (overlap as f64) / (anchor_size as f64)
            } else {
                0.0
            };

            // Weighted score: distance - (alignment_bonus)
            // Lower score = better match
            // alignment_factor ranges 0.0-1.0, bonus constant = 100
            // Use abs() to prioritize elements closer to the edge (whether inside or outside)
            // This also ensures outside elements (small +ve) are preferred over deep inside elements (large -ve)
            // Score calculation
            let score = (edge_dist.abs() as f64) - (alignment_factor * 100.0);
            let is_well_aligned = alignment_factor > 0.5;

            Some((candidate, is_well_aligned, score, alignment_factor))
        })
        .collect();

    // Sort by:
    // 1. is_well_aligned DESC (well-aligned elements first)
    // 2. score ASC (lower score = better)
    // 3. alignment_factor DESC (more aligned = better for ties)
    scored_candidates.sort_by(|a, b| {
        // First: is_well_aligned (true before false)
        match b.1.cmp(&a.1) {
            std::cmp::Ordering::Equal => {
                // Second: score (lower first)
                match a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal) {
                    std::cmp::Ordering::Equal => {
                        // Third: alignment_factor (higher first)
                        b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    other => other,
                }
            }
            other => other,
        }
    });

    scored_candidates
        .into_iter()
        .map(|(e, _, _, _)| e)
        .collect()
}

/// Find nth element matching regex pattern on resource ID
pub fn find_nth_by_id_regex<'a>(
    elements: &'a [UiElement],
    pattern: &str,
    index: u32,
) -> Option<&'a UiElement> {
    match Regex::new(pattern) {
        Ok(re) => elements
            .iter()
            .filter(|e| re.is_match(&e.resource_id))
            .nth(index as usize),
        Err(e) => {
            eprintln!("Invalid regex '{}': {}", pattern, e);
            None
        }
    }
}

pub fn find_all_by_id_regex<'a>(elements: &'a [UiElement], pattern: &str) -> Vec<&'a UiElement> {
    match Regex::new(pattern) {
        Ok(re) => elements
            .iter()
            .filter(|e| re.is_match(&e.resource_id))
            .collect(),
        Err(_) => Vec::new(),
    }
}

impl Bounds {
    /// Check if this bounds contains another bounds entirely
    pub fn contains(&self, other: &Bounds) -> bool {
        self.left <= other.left
            && self.top <= other.top
            && self.right >= other.right
            && self.bottom >= other.bottom
    }
}

/// Find parent element that contains a matching child element
/// Returns the parent element if found
pub fn find_parent_with_child<'a>(
    elements: &'a [UiElement],
    parent_matcher: impl Fn(&UiElement) -> bool,
    child_matcher: impl Fn(&UiElement) -> bool,
) -> Option<&'a UiElement> {
    // Find all potential parents
    let parents: Vec<_> = elements.iter().filter(|e| parent_matcher(e)).collect();

    // Find all potential children
    let children: Vec<_> = elements.iter().filter(|e| child_matcher(e)).collect();

    // For each parent, check if any child is contained within its bounds
    for parent in parents {
        for child in &children {
            if parent.bounds.contains(&child.bounds)
                && !std::ptr::eq(parent as *const _, *child as *const _)
            {
                return Some(parent);
            }
        }
    }

    None
}

/// Find nth element match regex pattern on content description
pub fn find_nth_by_description_regex<'a>(
    elements: &'a [UiElement],
    pattern: &str,
    index: u32,
) -> Option<&'a UiElement> {
    match Regex::new(pattern) {
        Ok(re) => elements
            .iter()
            .filter(|e| re.is_match(&e.content_desc))
            .nth(index as usize),
        Err(e) => {
            eprintln!("Invalid regex '{}': {}", pattern, e);
            None
        }
    }
}

pub fn find_all_by_description_regex<'a>(
    elements: &'a [UiElement],
    pattern: &str,
) -> Vec<&'a UiElement> {
    match Regex::new(pattern) {
        Ok(re) => elements
            .iter()
            .filter(|e| re.is_match(&e.content_desc))
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_html_entities_named() {
        assert_eq!(
            decode_html_entities("Devices &amp; Groups"),
            "Devices & Groups"
        );
        assert_eq!(decode_html_entities("&lt;tag&gt;"), "<tag>");
        assert_eq!(decode_html_entities("&quot;quoted&quot;"), "\"quoted\"");
        assert_eq!(decode_html_entities("it&apos;s"), "it's");
    }

    #[test]
    fn test_decode_html_entities_numeric() {
        assert_eq!(decode_html_entities("Security&#10;Safe"), "Security\nSafe");
        assert_eq!(decode_html_entities("line&#13;&#10;break"), "line\r\nbreak");
        assert_eq!(decode_html_entities("&#65;&#66;&#67;"), "ABC");
    }

    #[test]
    fn test_decode_html_entities_hex() {
        assert_eq!(decode_html_entities("&#x41;&#x42;&#x43;"), "ABC");
        assert_eq!(decode_html_entities("&#x0A;"), "\n");
    }

    #[test]
    fn test_decode_html_entities_mixed() {
        assert_eq!(
            decode_html_entities("Devices &amp; Groups&#10;2 devices on"),
            "Devices & Groups\n2 devices on"
        );
    }

    #[test]
    fn test_decode_html_entities_no_entities() {
        assert_eq!(decode_html_entities("Normal text"), "Normal text");
        assert_eq!(decode_html_entities(""), "");
    }

    #[test]
    fn test_parse_hierarchy_decodes_entities() {
        let xml = r#"<?xml version='1.0'?><hierarchy><node class="Button" text="" content-desc="Devices &amp; Groups" bounds="[0,0][100,100]" clickable="true" enabled="true" focusable="true"/></hierarchy>"#;
        let elements = parse_hierarchy(xml).unwrap();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].content_desc, "Devices & Groups");
    }

    #[test]
    fn test_parse_hierarchy_decodes_newline() {
        let xml = r#"<?xml version='1.0'?><hierarchy><node class="View" text="Security&#10;Safe" content-desc="" bounds="[0,0][100,100]" clickable="false" enabled="true" focusable="false"/></hierarchy>"#;
        let elements = parse_hierarchy(xml).unwrap();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].text, "Security\nSafe");
    }
}
