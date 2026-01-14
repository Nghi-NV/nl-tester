// GPS file parser module
// Supports: GPX (Lockito, Strava), KML (Google Maps), JSON (Google Takeout)

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use quick_xml::events::Event;
use quick_xml::Reader;

/// A single GPS coordinate point with optional metadata
#[derive(Debug, Clone)]
pub struct GpsPoint {
    pub lat: f64,
    pub lon: f64,
    pub altitude: Option<f64>,
    pub timestamp: Option<DateTime<Utc>>,
    pub speed: Option<f64>, // m/s
}

impl GpsPoint {
    pub fn new(lat: f64, lon: f64) -> Self {
        Self {
            lat,
            lon,
            altitude: None,
            timestamp: None,
            speed: None,
        }
    }
}

/// Parse GPX file content into GPS points
/// GPX format: <trkpt lat="x" lon="y"><time>ISO8601</time><ele>altitude</ele></trkpt>
pub fn parse_gpx(content: &str) -> Result<Vec<GpsPoint>> {
    let mut reader = Reader::from_str(content);
    // quick-xml 0.31 uses trim_text on reader directly, skip config for compatibility

    let mut points = Vec::new();
    let mut current_point: Option<GpsPoint> = None;
    let mut in_time = false;
    let mut in_ele = false;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                b"trkpt" | b"wpt" => {
                    let mut lat = 0.0;
                    let mut lon = 0.0;

                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"lat" => {
                                lat = std::str::from_utf8(&attr.value)
                                    .unwrap_or("0")
                                    .parse()
                                    .unwrap_or(0.0);
                            }
                            b"lon" => {
                                lon = std::str::from_utf8(&attr.value)
                                    .unwrap_or("0")
                                    .parse()
                                    .unwrap_or(0.0);
                            }
                            _ => {}
                        }
                    }

                    current_point = Some(GpsPoint::new(lat, lon));
                }
                b"time" => in_time = true,
                b"ele" => in_ele = true,
                _ => {}
            },
            Ok(Event::Text(ref e)) => {
                if let Some(ref mut point) = current_point {
                    let text = e.unescape().unwrap_or_default();

                    if in_time {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(&text) {
                            point.timestamp = Some(dt.with_timezone(&Utc));
                        }
                    }
                    if in_ele {
                        if let Ok(alt) = text.parse::<f64>() {
                            point.altitude = Some(alt);
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"trkpt" | b"wpt" => {
                    if let Some(point) = current_point.take() {
                        points.push(point);
                    }
                }
                b"time" => in_time = false,
                b"ele" => in_ele = false,
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("GPX parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    if points.is_empty() {
        return Err(anyhow::anyhow!("No GPS points found in GPX file"));
    }

    // Calculate speed between points if timestamps exist
    calculate_speeds(&mut points);

    Ok(points)
}

/// Parse KML file content (Google Maps export)
/// KML format: <coordinates>lon,lat,alt lon,lat,alt ...</coordinates>
pub fn parse_kml(content: &str) -> Result<Vec<GpsPoint>> {
    let mut reader = Reader::from_str(content);
    // quick-xml 0.31 uses trim_text on reader directly, skip config for compatibility

    let mut points = Vec::new();
    let mut in_coordinates = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == b"coordinates" {
                    in_coordinates = true;
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_coordinates {
                    let text = e.unescape().unwrap_or_default();
                    // Format: "lon,lat,alt lon,lat,alt" or one per line
                    for coord_str in text.split_whitespace() {
                        let parts: Vec<&str> = coord_str.trim().split(',').collect();
                        if parts.len() >= 2 {
                            let lon: f64 = parts[0].parse().unwrap_or(0.0);
                            let lat: f64 = parts[1].parse().unwrap_or(0.0);
                            let alt: Option<f64> = parts.get(2).and_then(|s| s.parse().ok());

                            let mut point = GpsPoint::new(lat, lon);
                            point.altitude = alt;
                            points.push(point);
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"coordinates" {
                    in_coordinates = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("KML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    if points.is_empty() {
        return Err(anyhow::anyhow!("No GPS points found in KML file"));
    }

    Ok(points)
}

/// Parse Google Takeout JSON (Records.json or Timeline.json)
/// Format: [{latitudeE7, longitudeE7, timestampMs, altitude}]
pub fn parse_google_json(content: &str) -> Result<Vec<GpsPoint>> {
    let json: serde_json::Value = serde_json::from_str(content).context("Invalid JSON format")?;

    let mut points = Vec::new();

    // Try Records.json format (array of locations)
    if let Some(locations) = json.get("locations").and_then(|l| l.as_array()) {
        for loc in locations {
            if let (Some(lat_e7), Some(lon_e7)) = (
                loc.get("latitudeE7").and_then(|v| v.as_i64()),
                loc.get("longitudeE7").and_then(|v| v.as_i64()),
            ) {
                let lat = lat_e7 as f64 / 1e7;
                let lon = lon_e7 as f64 / 1e7;
                let mut point = GpsPoint::new(lat, lon);

                if let Some(ts) = loc.get("timestampMs").and_then(|v| v.as_str()) {
                    if let Ok(ms) = ts.parse::<i64>() {
                        point.timestamp = DateTime::from_timestamp_millis(ms);
                    }
                }

                if let Some(alt) = loc.get("altitude").and_then(|v| v.as_f64()) {
                    point.altitude = Some(alt);
                }

                points.push(point);
            }
        }
    }

    // Try Timeline.json format (semanticSegments)
    if points.is_empty() {
        if let Some(segments) = json.get("semanticSegments").and_then(|s| s.as_array()) {
            for seg in segments {
                // Extract from timelinePath
                if let Some(path) = seg.get("timelinePath").and_then(|p| p.as_array()) {
                    for point_obj in path {
                        if let Some(point_str) = point_obj.get("point").and_then(|p| p.as_str()) {
                            // Format: "geo:lat,lon"
                            if let Some(coords) = point_str.strip_prefix("geo:") {
                                let parts: Vec<&str> = coords.split(',').collect();
                                if parts.len() >= 2 {
                                    let lat: f64 = parts[0].parse().unwrap_or(0.0);
                                    let lon: f64 = parts[1].parse().unwrap_or(0.0);
                                    points.push(GpsPoint::new(lat, lon));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if points.is_empty() {
        return Err(anyhow::anyhow!("No GPS points found in JSON file"));
    }

    calculate_speeds(&mut points);

    Ok(points)
}

/// Calculate speed between consecutive points based on timestamps
fn calculate_speeds(points: &mut [GpsPoint]) {
    for i in 1..points.len() {
        if let (Some(t1), Some(t2)) = (points[i - 1].timestamp, points[i].timestamp) {
            let duration_secs = (t2 - t1).num_milliseconds() as f64 / 1000.0;
            if duration_secs > 0.0 {
                let distance = haversine_distance(
                    points[i - 1].lat,
                    points[i - 1].lon,
                    points[i].lat,
                    points[i].lon,
                );
                points[i].speed = Some(distance / duration_secs);
            }
        }
    }
}

/// Calculate distance between two GPS points in meters using Haversine formula
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6371000.0; // Earth radius in meters

    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let delta_phi = (lat2 - lat1).to_radians();
    let delta_lambda = (lon2 - lon1).to_radians();

    let a = (delta_phi / 2.0).sin().powi(2)
        + phi1.cos() * phi2.cos() * (delta_lambda / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    R * c
}

/// Auto-detect format and parse GPS file
pub fn parse_gps_file(content: &str, extension: &str) -> Result<Vec<GpsPoint>> {
    match extension.to_lowercase().as_str() {
        "gpx" => parse_gpx(content),
        "kml" => parse_kml(content),
        "json" => parse_google_json(content),
        _ => Err(anyhow::anyhow!(
            "Unsupported GPS file format: {}",
            extension
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gpx() {
        let gpx = r#"<?xml version="1.0"?>
        <gpx version="1.1">
            <trk><trkseg>
                <trkpt lat="10.762622" lon="106.660172">
                    <ele>10.5</ele>
                    <time>2024-01-01T10:00:00Z</time>
                </trkpt>
                <trkpt lat="10.763000" lon="106.661000">
                    <ele>11.0</ele>
                    <time>2024-01-01T10:00:10Z</time>
                </trkpt>
            </trkseg></trk>
        </gpx>"#;

        let points = parse_gpx(gpx).unwrap();
        assert_eq!(points.len(), 2);
        assert!((points[0].lat - 10.762622).abs() < 0.0001);
        assert!(points[0].altitude.is_some());
    }

    #[test]
    fn test_haversine() {
        // Ho Chi Minh City to Hanoi ~1140km
        let dist = haversine_distance(10.762622, 106.660172, 21.028511, 105.804817);
        assert!(dist > 1_100_000.0 && dist < 1_200_000.0);
    }
}
