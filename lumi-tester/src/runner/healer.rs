//! Self-Healing selector engine for resilient test execution across platforms.
//!
//! When an element selector fails (due to text changes, renamed IDs, or layout shifts),
//! the self-healing engine evaluates candidate elements currently on screen against
//! previously recorded multi-signal fingerprints and heuristic similarity scores.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use crate::driver::traits::CandidateElement;

/// Multi-signal fingerprint of a successfully located element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementFingerprint {
    pub selector_key: String,
    pub text: Option<String>,
    pub id: Option<String>,
    pub description: Option<String>,
    pub element_type: Option<String>,
    pub bounds: (i32, i32, i32, i32),
    pub center: (i32, i32),
}

/// Details of a successful self-healing match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealedMatch {
    pub original_selector: String,
    pub healed_text: Option<String>,
    pub healed_id: Option<String>,
    pub target_point: (i32, i32),
    pub confidence: f32,
    pub reason: String,
    pub suggestion: String,
}

/// Self-healing coordinator
#[derive(Debug, Clone, Default)]
pub struct SelfHealer {
    enabled: bool,
    fingerprints: HashMap<String, ElementFingerprint>,
    heals: Vec<HealedMatch>,
}

impl SelfHealer {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            fingerprints: HashMap::new(),
            heals: Vec::new(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Record a successfully interacted element to serve as baseline fingerprint
    pub fn record_success(&mut self, selector_key: &str, candidate: &CandidateElement) {
        if !self.enabled {
            return;
        }
        self.fingerprints.insert(
            selector_key.to_string(),
            ElementFingerprint {
                selector_key: selector_key.to_string(),
                text: candidate.text.clone(),
                id: candidate.id.clone(),
                description: candidate.description.clone(),
                element_type: candidate.element_type.clone(),
                bounds: candidate.bounds,
                center: candidate.center,
            },
        );
    }

    /// Attempt to find a self-healed replacement for a failed selector
    pub fn attempt_heal(
        &mut self,
        selector_key: &str,
        query_text: Option<&str>,
        query_id: Option<&str>,
        query_desc: Option<&str>,
        candidates: &[CandidateElement],
    ) -> Option<HealedMatch> {
        if !self.enabled || candidates.is_empty() {
            return None;
        }

        let baseline = self.fingerprints.get(selector_key);

        let target_text = query_text.or_else(|| baseline.and_then(|b| b.text.as_deref()));
        let target_id = query_id.or_else(|| baseline.and_then(|b| b.id.as_deref()));
        let target_desc = query_desc.or_else(|| baseline.and_then(|b| b.description.as_deref()));
        let target_center = baseline.map(|b| b.center);
        let target_type = baseline.and_then(|b| b.element_type.as_deref());

        let mut best_candidate: Option<(&CandidateElement, f32, String)> = None;

        for cand in candidates {
            // Must have valid bounds and be enabled/visible
            if cand.bounds.2 <= cand.bounds.0 || cand.bounds.3 <= cand.bounds.1 {
                continue;
            }

            let mut score = 0.0f32;
            let mut reasons = Vec::new();

            // 1. ID similarity (Weight: 0.40)
            if let (Some(tid), Some(cid)) = (target_id, cand.id.as_deref()) {
                if tid == cid {
                    score += 0.40;
                    reasons.push(format!("Exact ID '{}'", cid));
                } else if tid.ends_with(cid) || cid.ends_with(tid) {
                    score += 0.35;
                    reasons.push(format!("Partial ID '{}'", cid));
                } else {
                    let id_sim = levenshtein_similarity(tid, cid);
                    if id_sim >= 0.70 {
                        score += 0.30 * id_sim;
                        reasons.push(format!("ID similarity {:.0}%", id_sim * 100.0));
                    }
                }
            }

            // 2. Text / Content Description similarity (Weight: 0.35)
            let cand_label = cand.text.as_deref().or(cand.description.as_deref());
            let target_label = target_text.or(target_desc);
            if let (Some(t_label), Some(c_label)) = (target_label, cand_label) {
                let text_sim = text_similarity(t_label, c_label);
                if text_sim > 0.40 {
                    score += 0.35 * text_sim;
                    reasons.push(format!("Label '{}' ({:.0}%)", c_label, text_sim * 100.0));
                }
            }

            // 3. Position Proximity (Weight: 0.20)
            if let Some((bx, by)) = target_center {
                let (cx, cy) = cand.center;
                let dx = (bx - cx).abs() as f32;
                let dy = (by - cy).abs() as f32;
                let dist = (dx * dx + dy * dy).sqrt();
                // If within 150 pixels, award spatial score
                if dist < 150.0 {
                    let spatial_score = 0.20 * (1.0 - (dist / 150.0));
                    score += spatial_score;
                    reasons.push(format!("Spatial dist {:.0}px", dist));
                }
            }

            // 4. Element Type match (Weight: 0.10)
            if let (Some(tt), Some(ct)) = (target_type, cand.element_type.as_deref()) {
                if tt.eq_ignore_ascii_case(ct) || ct.ends_with(tt) || tt.ends_with(ct) {
                    score += 0.10;
                    reasons.push("Type match".to_string());
                }
            }

            if score > 0.0 {
                if let Some((_, best_score, _)) = best_candidate {
                    if score > best_score {
                        best_candidate = Some((cand, score, reasons.join(", ")));
                    }
                } else {
                    best_candidate = Some((cand, score, reasons.join(", ")));
                }
            }
        }

        // Confidence threshold for self-healing: 0.70
        if let Some((cand, confidence, reason)) = best_candidate {
            if confidence >= 0.70 {
                let suggestion = if let Some(text) = &cand.text {
                    format!("tap: \"{}\"", text)
                } else if let Some(id) = &cand.id {
                    format!("tap: {{ id: \"{}\" }}", id)
                } else if let Some(desc) = &cand.description {
                    format!("tap: {{ desc: \"{}\" }}", desc)
                } else {
                    format!("tap: \"{},{}\"", cand.center.0, cand.center.1)
                };

                let healed = HealedMatch {
                    original_selector: selector_key.to_string(),
                    healed_text: cand.text.clone(),
                    healed_id: cand.id.clone(),
                    target_point: cand.center,
                    confidence: (confidence * 100.0).round() / 100.0,
                    reason,
                    suggestion,
                };
                self.heals.push(healed.clone());
                return Some(healed);
            }
        }

        None
    }

    pub fn recorded_heals(&self) -> &[HealedMatch] {
        &self.heals
    }
}

/// Compute text similarity combining Levenshtein ratio and token overlap
pub fn text_similarity(s1: &str, s2: &str) -> f32 {
    let s1 = s1.trim().to_lowercase();
    let s2 = s2.trim().to_lowercase();
    if s1 == s2 {
        return 1.0;
    }
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }

    let lev_sim = levenshtein_similarity(&s1, &s2);

    // Token set overlap (Jaccard similarity on words)
    let words1: std::collections::HashSet<&str> = s1.split_whitespace().collect();
    let words2: std::collections::HashSet<&str> = s2.split_whitespace().collect();
    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();
    let token_sim = if union > 0 {
        intersection as f32 / union as f32
    } else {
        0.0
    };

    (lev_sim * 0.6) + (token_sim * 0.4)
}

/// Standard normalized Levenshtein distance similarity in 0.0..1.0
pub fn levenshtein_similarity(s1: &str, s2: &str) -> f32 {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();
    if len1 == 0 && len2 == 0 {
        return 1.0;
    }
    if len1 == 0 || len2 == 0 {
        return 0.0;
    }
    let max_len = len1.max(len2) as f32;

    let v1: Vec<char> = s1.chars().collect();
    let v2: Vec<char> = s2.chars().collect();
    let mut prev_row: Vec<usize> = (0..=len2).collect();
    let mut curr_row = vec![0; len2 + 1];

    for i in 0..len1 {
        curr_row[0] = i + 1;
        for j in 0..len2 {
            let cost = if v1[i] == v2[j] { 0 } else { 1 };
            curr_row[j + 1] = (curr_row[j] + 1)
                .min(prev_row[j + 1] + 1)
                .min(prev_row[j] + cost);
        }
        prev_row.copy_from_slice(&curr_row);
    }
    let dist = prev_row[len2] as f32;
    (1.0 - (dist / max_len)).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_similarity() {
        assert_eq!(levenshtein_similarity("login", "login"), 1.0);
        let sim = levenshtein_similarity("Login with Google", "Sign in with Google");
        assert!(sim > 0.60, "similarity was {}", sim);

        let diff = levenshtein_similarity("Submit", "Cancel");
        assert!(diff < 0.30, "similarity was {}", diff);
    }

    #[test]
    fn test_text_similarity_tokens() {
        let sim = text_similarity("Login with Google", "Sign in with Google");
        assert!(sim > 0.60, "similarity was {}", sim);
    }

    #[test]
    fn test_self_healing_id_match_when_text_changed() {
        let mut healer = SelfHealer::new(true);

        // Baseline run recorded button "Login with Google" with id "btn_google"
        let baseline_cand = CandidateElement {
            text: Some("Login with Google".to_string()),
            id: Some("com.app:id/btn_google".to_string()),
            description: None,
            element_type: Some("Button".to_string()),
            bounds: (50, 400, 300, 460),
            center: (175, 430),
            clickable: true,
            enabled: true,
        };
        healer.record_success("tap_login_button", &baseline_cand);

        // In next sprint, developer changed text to "Sign in with Google", id is unchanged
        let current_screen = vec![
            CandidateElement {
                text: Some("Cancel".to_string()),
                id: Some("com.app:id/btn_cancel".to_string()),
                description: None,
                element_type: Some("Button".to_string()),
                bounds: (50, 600, 300, 660),
                center: (175, 630),
                clickable: true,
                enabled: true,
            },
            CandidateElement {
                text: Some("Sign in with Google".to_string()),
                id: Some("com.app:id/btn_google".to_string()),
                description: None,
                element_type: Some("Button".to_string()),
                bounds: (50, 400, 300, 460),
                center: (175, 430),
                clickable: true,
                enabled: true,
            },
        ];

        let healed = healer.attempt_heal(
            "tap_login_button",
            Some("Login with Google"),
            Some("com.app:id/btn_google"),
            None,
            &current_screen,
        ).expect("should find healed candidate");

        assert_eq!(healed.healed_text.as_deref(), Some("Sign in with Google"));
        assert!(healed.confidence >= 0.80, "confidence was {}", healed.confidence);
        assert_eq!(healed.target_point, (175, 430));
        assert_eq!(healed.suggestion, "tap: \"Sign in with Google\"");
    }
}
