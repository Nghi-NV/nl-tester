use super::bridge::MacosBridge;
use crate::driver::traits::Selector;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Strongly typed Accessibility UI Element node (equivalent to UiElement in UiAutomator)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AXNode {
    pub role: String,
    pub title: String,
    pub description: String,
    pub value: String,
    pub identifier: String,
    #[serde(default)]
    pub placeholder: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub children: Vec<AXNode>,
}

impl AXNode {
    pub fn matches_text(&self, text: &str, exact: bool) -> bool {
        let text_lower = text.to_lowercase();
        let fields = [&self.title, &self.value, &self.description, &self.placeholder];
        if exact {
            fields.iter().any(|f| f.eq_ignore_ascii_case(text))
        } else {
            fields.iter().any(|f| f.to_lowercase().contains(&text_lower))
        }
    }

    pub fn matches_regex(&self, regex: &Regex) -> bool {
        let fields = [&self.title, &self.value, &self.description, &self.placeholder];
        fields.iter().any(|f| !f.is_empty() && regex.is_match(f))
    }

    pub fn matches_id(&self, id: &str) -> bool {
        self.identifier.eq_ignore_ascii_case(id)
    }

    pub fn matches_id_regex(&self, regex: &Regex) -> bool {
        !self.identifier.is_empty() && regex.is_match(&self.identifier)
    }

    pub fn matches_type(&self, elem_type: &str) -> bool {
        let req = elem_type.to_lowercase();
        let act = self.role.to_lowercase();

        match req.as_str() {
            "input" | "textfield" | "text_field" | "edit" | "edittext" | "textbox" | "text_box" | "textarea" | "text_area" => {
                act.contains("textfield")
                    || act.contains("edittext")
                    || act.contains("textarea")
                    || act.contains("textbox")
                    || act == "edit"
                    || act.contains("securetextfield")
                    || act == "input"
            }
            "button" | "btn" => act.contains("button") || act == "btn",
            "text" | "label" | "statictext" | "static_text" | "textview" => {
                act.contains("statictext")
                    || act.contains("textview")
                    || act.contains("label")
                    || act == "text"
            }
            "switch" | "toggle" => act.contains("switch") || act.contains("toggle"),
            "checkbox" | "check_box" => act.contains("checkbox") || act.contains("check_box"),
            "radio" | "radiobutton" | "radio_button" => act.contains("radio"),
            "image" | "img" | "icon" | "imageview" => {
                act.contains("image") || act.contains("icon") || act == "img"
            }
            "select" | "dropdown" | "combobox" | "combo_box" | "popup" | "popupbutton" | "spinner" => {
                act.contains("popupbutton")
                    || act.contains("combobox")
                    || act.contains("spinner")
                    || act.contains("dropdown")
                    || act.contains("select")
            }
            "slider" | "seekbar" => act.contains("slider") || act.contains("seekbar"),
            "list" | "table" | "row" | "cell" | "item" | "group" => act.contains(&req),
            _ => self.role.eq_ignore_ascii_case(elem_type) || act.contains(&req),
        }
    }

    pub fn center_point(&self) -> (i32, i32) {
        (
            (self.x + self.width / 2.0).round() as i32,
            (self.y + self.height / 2.0).round() as i32,
        )
    }
}

/// Accessibility query and action engine for macOS applications (similar to UiAutomator)
#[derive(Debug, Default, Clone)]
pub struct MacosAccessibility;

impl MacosAccessibility {
    pub fn new() -> Self {
        Self
    }

    /// Dump full UI hierarchy of target application as XML string
    pub fn dump_ui_hierarchy(&self, app_target: &str) -> Result<String> {
        const SCRIPT: &str = r#"
import AppKit
import ApplicationServices
import Foundation

guard CommandLine.arguments.count > 1 else { exit(1) }
let appParam = CommandLine.arguments[1]

var targetApp: NSRunningApplication?
if !appParam.isEmpty {
    let appsByBundle = NSRunningApplication.runningApplications(withBundleIdentifier: appParam)
    if let first = appsByBundle.first {
        targetApp = first
    } else {
        let allApps = NSWorkspace.shared.runningApplications
        for app in allApps {
            guard app.activationPolicy == .regular else { continue }
            if let bundleId = app.bundleIdentifier, bundleId.caseInsensitiveCompare(appParam) == .orderedSame {
                targetApp = app
                break
            }
            if let url = app.bundleURL, (url.path.caseInsensitiveCompare(appParam) == .orderedSame || url.lastPathComponent.caseInsensitiveCompare(appParam) == .orderedSame) {
                targetApp = app
                break
            }
            if let name = app.localizedName, (name.caseInsensitiveCompare(appParam) == .orderedSame || appParam.hasSuffix(name)) {
                targetApp = app
                break
            }
        }
    }
}
if targetApp == nil {
    targetApp = NSWorkspace.shared.frontmostApplication
}
guard let app = targetApp else {
    print("<hierarchy platform=\"macos\"/>")
    exit(0)
}

let appElement = AXUIElementCreateApplication(app.processIdentifier)

func xmlEscape(_ text: String) -> String {
    return text
        .replacingOccurrences(of: "&", with: "&amp;")
        .replacingOccurrences(of: "<", with: "&lt;")
        .replacingOccurrences(of: ">", with: "&gt;")
        .replacingOccurrences(of: "\"", with: "&quot;")
        .replacingOccurrences(of: "'", with: "&apos;")
}

func dumpNode(_ element: AXUIElement, depth: Int) {
    var roleVal: CFTypeRef?
    var titleVal: CFTypeRef?
    var descVal: CFTypeRef?
    var valVal: CFTypeRef?
    var idVal: CFTypeRef?
    var posVal: CFTypeRef?
    var sizeVal: CFTypeRef?
    var placeholderVal: CFTypeRef?

    _ = AXUIElementCopyAttributeValue(element, kAXRoleAttribute as CFString, &roleVal)
    _ = AXUIElementCopyAttributeValue(element, kAXTitleAttribute as CFString, &titleVal)
    _ = AXUIElementCopyAttributeValue(element, kAXDescriptionAttribute as CFString, &descVal)
    _ = AXUIElementCopyAttributeValue(element, kAXValueAttribute as CFString, &valVal)
    _ = AXUIElementCopyAttributeValue(element, kAXIdentifierAttribute as CFString, &idVal)
    _ = AXUIElementCopyAttributeValue(element, kAXPositionAttribute as CFString, &posVal)
    _ = AXUIElementCopyAttributeValue(element, kAXSizeAttribute as CFString, &sizeVal)
    _ = AXUIElementCopyAttributeValue(element, "AXPlaceholderValue" as CFString, &placeholderVal)

    var point = CGPoint.zero
    var size = CGSize.zero
    if let p = posVal { AXValueGetValue(p as! AXValue, .cgPoint, &point) }
    if let s = sizeVal { AXValueGetValue(s as! AXValue, .cgSize, &size) }

    let role = roleVal != nil ? "\(roleVal!)" : "AXUnknown"
    let title = titleVal != nil ? "\(titleVal!)" : ""
    let desc = descVal != nil ? "\(descVal!)" : ""
    let val = valVal != nil ? "\(valVal!)" : ""
    let ident = idVal != nil ? "\(idVal!)" : ""
    let placeholder = placeholderVal != nil ? "\(placeholderVal!)" : ""

    let indent = String(repeating: "  ", count: depth + 1)
    print("\(indent)<element role=\"\(xmlEscape(role))\" title=\"\(xmlEscape(title))\" description=\"\(xmlEscape(desc))\" value=\"\(xmlEscape(val))\" placeholder=\"\(xmlEscape(placeholder))\" id=\"\(xmlEscape(ident))\" x=\"\(Int(point.x))\" y=\"\(Int(point.y))\" width=\"\(Int(size.width))\" height=\"\(Int(size.height))\"/>")

    var childrenVal: CFTypeRef?
    if AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenVal) == .success,
       let children = childrenVal as? [AXUIElement] {
        for child in children {
            dumpNode(child, depth: depth + 1)
        }
    }
}

print("<hierarchy platform=\"macos\">")
var winVal: CFTypeRef?
if AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &winVal) == .success,
   let windows = winVal as? [AXUIElement], !windows.isEmpty {
    for win in windows {
        dumpNode(win, depth: 0)
    }
} else {
    dumpNode(appElement, depth: 0)
}
print("</hierarchy>")
"#;
        MacosBridge::run_swift(SCRIPT, &[app_target])
    }

    /// Parse raw XML hierarchy into flat list of AXNodes
    pub fn parse_hierarchy_nodes(&self, xml: &str) -> Vec<AXNode> {
        let mut nodes = Vec::new();
        for line in xml.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("<element ") {
                continue;
            }
            let extract_attr = |attr_name: &str| -> String {
                let pattern = format!("{}=\"", attr_name);
                if let Some(start) = trimmed.find(&pattern) {
                    let after = &trimmed[start + pattern.len()..];
                    if let Some(end) = after.find('"') {
                        return after[..end]
                            .replace("&quot;", "\"")
                            .replace("&apos;", "'")
                            .replace("&lt;", "<")
                            .replace("&gt;", ">")
                            .replace("&amp;", "&");
                    }
                }
                String::new()
            };

            let role = extract_attr("role");
            let title = extract_attr("title");
            let description = extract_attr("description");
            let value = extract_attr("value");
            let placeholder = extract_attr("placeholder");
            let identifier = extract_attr("id");
            let x = extract_attr("x").parse::<f64>().unwrap_or(0.0);
            let y = extract_attr("y").parse::<f64>().unwrap_or(0.0);
            let width = extract_attr("width").parse::<f64>().unwrap_or(0.0);
            let height = extract_attr("height").parse::<f64>().unwrap_or(0.0);

            nodes.push(AXNode {
                role,
                title,
                description,
                value,
                placeholder,
                identifier,
                x,
                y,
                width,
                height,
                actions: Vec::new(),
                children: Vec::new(),
            });
        }
        nodes
    }

    /// Find all nodes matching a basic selector
    pub fn find_nodes_matching<'a>(
        &self,
        nodes: &'a [AXNode],
        selector: &Selector,
    ) -> Vec<&'a AXNode> {
        match selector {
            Selector::Text(text, _, exact) => {
                nodes.iter().filter(|n| n.matches_text(text, *exact)).collect()
            }
            Selector::TextRegex(pattern, _) => match Regex::new(pattern) {
                Ok(regex) => nodes.iter().filter(|n| n.matches_regex(&regex)).collect(),
                Err(_) => Vec::new(),
            },
            Selector::Id(id, _) => nodes.iter().filter(|n| n.matches_id(id)).collect(),
            Selector::IdRegex(pattern, _) => match Regex::new(pattern) {
                Ok(regex) => nodes.iter().filter(|n| n.matches_id_regex(&regex)).collect(),
                Err(_) => Vec::new(),
            },
            Selector::Type(elem_type, _) | Selector::Role(elem_type, _) => {
                nodes.iter().filter(|n| n.matches_type(elem_type)).collect()
            }
            Selector::Description(desc, _) | Selector::Placeholder(desc, _) => {
                nodes.iter().filter(|n| n.matches_text(desc, false)).collect()
            }
            Selector::DescriptionRegex(pattern, _) => match Regex::new(pattern) {
                Ok(regex) => nodes.iter().filter(|n| n.matches_regex(&regex)).collect(),
                Err(_) => Vec::new(),
            },
            Selector::AnyClickable(_) => nodes
                .iter()
                .filter(|n| {
                    n.matches_type("button")
                        || n.matches_type("switch")
                        || n.matches_type("checkbox")
                        || n.matches_type("input")
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Match elements spatially relative to an anchor element (leftOf, rightOf, above, below, near)
    pub fn find_relative<'a>(
        candidates: &[&'a AXNode],
        anchor: &AXNode,
        direction: crate::driver::traits::RelativeDirection,
        max_dist: Option<u32>,
    ) -> Option<&'a AXNode> {
        use crate::driver::traits::RelativeDirection;
        let limit = max_dist.map(|d| d as f64).unwrap_or(f64::MAX);

        let mut scored: Vec<(&'a AXNode, f64)> = candidates
            .iter()
            .filter_map(|&candidate| {
                if (candidate.x - anchor.x).abs() < 1.0
                    && (candidate.y - anchor.y).abs() < 1.0
                    && (candidate.width - anchor.width).abs() < 1.0
                {
                    return None;
                }

                let is_valid = match direction {
                    RelativeDirection::RightOf => {
                        candidate.x >= (anchor.x + anchor.width * 0.3)
                    }
                    RelativeDirection::LeftOf => {
                        (candidate.x + candidate.width) <= (anchor.x + anchor.width * 0.7)
                    }
                    RelativeDirection::Below => {
                        candidate.y >= (anchor.y + anchor.height * 0.3)
                    }
                    RelativeDirection::Above => {
                        (candidate.y + candidate.height) <= (anchor.y + anchor.height * 0.7)
                    }
                    RelativeDirection::Near => true,
                };

                if !is_valid {
                    return None;
                }

                let edge_dist = match direction {
                    RelativeDirection::RightOf => (candidate.x - (anchor.x + anchor.width)).max(0.0),
                    RelativeDirection::LeftOf => (anchor.x - (candidate.x + candidate.width)).max(0.0),
                    RelativeDirection::Below => (candidate.y - (anchor.y + anchor.height)).max(0.0),
                    RelativeDirection::Above => (anchor.y - (candidate.y + candidate.height)).max(0.0),
                    RelativeDirection::Near => {
                        let (ax, ay) = anchor.center_point();
                        let (cx, cy) = candidate.center_point();
                        (((cx - ax).pow(2) + (cy - ay).pow(2)) as f64).sqrt()
                    }
                };

                if edge_dist > limit {
                    return None;
                }

                let overlap = match direction {
                    RelativeDirection::RightOf | RelativeDirection::LeftOf => {
                        let top = candidate.y.max(anchor.y);
                        let bottom = (candidate.y + candidate.height).min(anchor.y + anchor.height);
                        (bottom - top).max(0.0)
                    }
                    RelativeDirection::Below | RelativeDirection::Above => {
                        let left = candidate.x.max(anchor.x);
                        let right = (candidate.x + candidate.width).min(anchor.x + anchor.width);
                        (right - left).max(0.0)
                    }
                    RelativeDirection::Near => 0.0,
                };

                let score = edge_dist - overlap * 3.0;
                Some((candidate, score))
            })
            .collect();

        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.first().map(|(node, _)| *node)
    }

    /// Find an element in target application matching a Selector
    pub fn find_element(&self, app_target: &str, selector: &Selector) -> Result<Option<AXNode>> {
        let xml = self.dump_ui_hierarchy(app_target)?;
        let nodes = self.parse_hierarchy_nodes(&xml);

        match selector {
            Selector::Relative {
                target,
                anchor,
                direction,
                max_dist,
            } => {
                let Some(anchor_node) = self.find_element(app_target, anchor)? else {
                    return Ok(None);
                };
                let candidates = self.find_nodes_matching(&nodes, target);
                if let Some(matched) = Self::find_relative(&candidates, &anchor_node, *direction, *max_dist) {
                    return Ok(Some(matched.clone()));
                }
                Ok(None)
            }
            _ => {
                let matches = self.find_nodes_matching(&nodes, selector);
                let target_idx = match selector {
                    Selector::Text(_, idx, _)
                    | Selector::TextRegex(_, idx)
                    | Selector::Id(_, idx)
                    | Selector::IdRegex(_, idx)
                    | Selector::Type(_, idx)
                    | Selector::Role(_, idx)
                    | Selector::Description(_, idx)
                    | Selector::DescriptionRegex(_, idx)
                    | Selector::Placeholder(_, idx)
                    | Selector::AnyClickable(idx) => *idx,
                    _ => 0,
                };

                if target_idx < matches.len() {
                    Ok(Some(matches[target_idx].clone()))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Perform semantic AXPress action directly on matching button or interactive element
    pub fn press_element(&self, app_target: &str, selector: &Selector) -> Result<bool> {
        const SCRIPT: &str = r#"
import AppKit
import ApplicationServices
import Foundation

guard CommandLine.arguments.count >= 3 else { exit(1) }
let appParam = CommandLine.arguments[1]
let query = CommandLine.arguments[2]
let targetIndex = CommandLine.arguments.count > 3 ? (Int(CommandLine.arguments[3]) ?? 0) : 0

var targetApp: NSRunningApplication?
if !appParam.isEmpty {
    let appsByBundle = NSRunningApplication.runningApplications(withBundleIdentifier: appParam)
    if let first = appsByBundle.first {
        targetApp = first
    } else {
        let allApps = NSWorkspace.shared.runningApplications
        for app in allApps {
            guard app.activationPolicy == .regular else { continue }
            if let bundleId = app.bundleIdentifier, bundleId.caseInsensitiveCompare(appParam) == .orderedSame {
                targetApp = app
                break
            }
            if let url = app.bundleURL, (url.path.caseInsensitiveCompare(appParam) == .orderedSame || url.lastPathComponent.caseInsensitiveCompare(appParam) == .orderedSame) {
                targetApp = app
                break
            }
            if let name = app.localizedName, (name.caseInsensitiveCompare(appParam) == .orderedSame || appParam.hasSuffix(name)) {
                targetApp = app
                break
            }
        }
    }
}
if targetApp == nil {
    targetApp = NSWorkspace.shared.frontmostApplication
}
guard let app = targetApp else { exit(1) }
let appElement = AXUIElementCreateApplication(app.processIdentifier)

func matchesExact(_ str: String) -> Bool {
    if str.isEmpty { return false }
    if str.caseInsensitiveCompare(query) == .orderedSame { return true }
    if query.contains("|") {
        let tokens = query.components(separatedBy: "|")
        for token in tokens where !token.isEmpty {
            if str.caseInsensitiveCompare(token) == .orderedSame { return true }
        }
    }
    return false
}

func matchesSubstring(_ str: String) -> Bool {
    if str.isEmpty { return false }
    if str.localizedCaseInsensitiveContains(query) { return true }
    if query.contains("|") {
        let tokens = query.components(separatedBy: "|")
        for token in tokens where !token.isEmpty {
            if str.localizedCaseInsensitiveContains(token) { return true }
        }
    }
    return false
}

var exactPressables: [AXUIElement] = []
var substringPressables: [AXUIElement] = []

func collectButtons(_ element: AXUIElement) {
    var titleVal: CFTypeRef?
    var descVal: CFTypeRef?
    var valVal: CFTypeRef?
    var idVal: CFTypeRef?
    
    var titleStr = ""
    var descStr = ""
    var valStr = ""
    var idStr = ""
    
    if AXUIElementCopyAttributeValue(element, kAXTitleAttribute as CFString, &titleVal) == .success, let unwrapped = titleVal {
        titleStr = "\(unwrapped)"
    }
    if AXUIElementCopyAttributeValue(element, kAXDescriptionAttribute as CFString, &descVal) == .success, let unwrapped = descVal {
        descStr = "\(unwrapped)"
    }
    if AXUIElementCopyAttributeValue(element, kAXValueAttribute as CFString, &valVal) == .success, let unwrapped = valVal {
        valStr = "\(unwrapped)"
    }
    if AXUIElementCopyAttributeValue(element, kAXIdentifierAttribute as CFString, &idVal) == .success, let unwrapped = idVal {
        idStr = "\(unwrapped)"
    }
    
    var actions: CFArray?
    var hasPress = false
    if AXUIElementCopyActionNames(element, &actions) == .success, let actionList = actions as? [String] {
        hasPress = actionList.contains("AXPress")
    }
    
    if hasPress {
        if matchesExact(titleStr) || matchesExact(descStr) || matchesExact(valStr) || matchesExact(idStr) {
            exactPressables.append(element)
        } else if matchesSubstring(titleStr) || matchesSubstring(descStr) || matchesSubstring(valStr) || matchesSubstring(idStr) {
            substringPressables.append(element)
        }
    }
    
    var childrenVal: CFTypeRef?
    if AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenVal) == .success,
       let children = childrenVal as? [AXUIElement] {
        for child in children {
            collectButtons(child)
        }
    }
}

var winVal: CFTypeRef?
if AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &winVal) == .success,
   let windows = winVal as? [AXUIElement] {
    for win in windows {
        collectButtons(win)
    }
}

let candidates = !exactPressables.isEmpty ? exactPressables : substringPressables
if !candidates.isEmpty {
    let target = targetIndex < candidates.count ? candidates[targetIndex] : candidates.last!
    let res = AXUIElementPerformAction(target, kAXPressAction as CFString)
    if res == .success {
        print("AX_PRESS_SUCCESS")
        exit(0)
    }
}
exit(1)
"#;
        let (query, target_idx) = match selector {
            Selector::Text(t, idx, _) => (t.clone(), *idx),
            Selector::TextRegex(r, idx) => (r.clone(), *idx),
            Selector::Id(id, idx) => (id.clone(), *idx),
            Selector::IdRegex(r, idx) => (r.clone(), *idx),
            Selector::Description(d, idx) | Selector::Placeholder(d, idx) => (d.clone(), *idx),
            Selector::DescriptionRegex(r, idx) => (r.clone(), *idx),
            _ => return Ok(false),
        };

        let idx_str = target_idx.to_string();
        if let Ok(out) = MacosBridge::run_swift(SCRIPT, &[app_target, &query, &idx_str]) {
            if out.contains("AX_PRESS_SUCCESS") {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Set input text on active or first visible text field with React/WebKit synthetic event support
    pub fn input_text(&self, app_target: &str, text: &str) -> Result<()> {
        const SCRIPT: &str = r#"
import AppKit
import ApplicationServices
import Foundation

guard CommandLine.arguments.count >= 3 else { exit(1) }
let appParam = CommandLine.arguments[1]
let newText = CommandLine.arguments[2]

var targetApp: NSRunningApplication?
if !appParam.isEmpty {
    let appsByBundle = NSRunningApplication.runningApplications(withBundleIdentifier: appParam)
    if let first = appsByBundle.first {
        targetApp = first
    } else {
        let allApps = NSWorkspace.shared.runningApplications
        for app in allApps {
            guard app.activationPolicy == .regular else { continue }
            if let bundleId = app.bundleIdentifier, bundleId.caseInsensitiveCompare(appParam) == .orderedSame {
                targetApp = app
                break
            }
            if let url = app.bundleURL, (url.path.caseInsensitiveCompare(appParam) == .orderedSame || url.lastPathComponent.caseInsensitiveCompare(appParam) == .orderedSame) {
                targetApp = app
                break
            }
            if let name = app.localizedName, (name.caseInsensitiveCompare(appParam) == .orderedSame || appParam.hasSuffix(name)) {
                targetApp = app
                break
            }
        }
    }
}
if targetApp == nil {
    targetApp = NSWorkspace.shared.frontmostApplication
}
guard let app = targetApp else { exit(1) }
let pid = app.processIdentifier
let appElement = AXUIElementCreateApplication(pid)

var targetField: AXUIElement?
var focusedElem: CFTypeRef?
if AXUIElementCopyAttributeValue(appElement, kAXFocusedUIElementAttribute as CFString, &focusedElem) == .success,
   let focused = focusedElem {
    targetField = (focused as! AXUIElement)
}

if targetField == nil {
    func findFirstTextField(_ element: AXUIElement) -> AXUIElement? {
        var roleVal: CFTypeRef?
        if AXUIElementCopyAttributeValue(element, kAXRoleAttribute as CFString, &roleVal) == .success, let unwrapped = roleVal {
            let r = "\(unwrapped)"
            if r.contains("TextField") || r.contains("TextArea") {
                return element
            }
        }
        var childrenVal: CFTypeRef?
        if AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenVal) == .success,
           let children = childrenVal as? [AXUIElement] {
            for child in children {
                if let found = findFirstTextField(child) { return found }
            }
        }
        return nil
    }
    var winVal: CFTypeRef?
    if AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &winVal) == .success,
       let windows = winVal as? [AXUIElement] {
        for win in windows {
            if let found = findFirstTextField(win) {
                targetField = found
                break
            }
        }
    }
}

if let field = targetField {
    _ = AXUIElementSetAttributeValue(field, kAXFocusedAttribute as CFString, kCFBooleanTrue)
    _ = AXUIElementSetAttributeValue(field, kAXValueAttribute as CFString, newText as CFTypeRef)
}

let src = CGEventSource(stateID: .hidSystemState)
if let cmdDown = CGEvent(keyboardEventSource: src, virtualKey: 55, keyDown: true),
   let aDown = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: true),
   let aUp = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: false),
   let cmdUp = CGEvent(keyboardEventSource: src, virtualKey: 55, keyDown: false) {
    aDown.flags = .maskCommand
    cmdDown.postToPid(pid)
    aDown.postToPid(pid)
    aUp.postToPid(pid)
    cmdUp.postToPid(pid)
    Thread.sleep(forTimeInterval: 0.02)
}

for char in newText.utf16 {
    var code = char
    if let down = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: true),
       let up = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: false) {
        down.keyboardSetUnicodeString(stringLength: 1, unicodeString: &code)
        up.keyboardSetUnicodeString(stringLength: 1, unicodeString: &code)
        down.postToPid(pid)
        up.postToPid(pid)
        Thread.sleep(forTimeInterval: 0.01)
    }
}

print("AX_SET_TEXT_SUCCESS")
exit(0)
"#;
        let res = MacosBridge::run_swift(SCRIPT, &[app_target, text]);
        if let Ok(out) = res {
            if out.contains("AX_SET_TEXT_SUCCESS") {
                return Ok(());
            }
        }
        // Fallback to osascript
        MacosBridge::run_osascript(&format!(
            "tell application \"System Events\" to keystroke \"{}\"",
            text.replace('"', "\\\"")
        ))?;
        Ok(())
    }
}
