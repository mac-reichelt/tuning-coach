//! Typed representation of the `recommendation` WebSocket payload.
//!
//! See `docs/adr/0003-phase3-recommendation-payload-extensions.md` for the
//! authoritative field-level reference and forward-compatibility rules.

use serde::{Deserialize, Serialize};

/// Tuning surface area that a recommendation targets.
///
/// Serialises as snake\_case (e.g. `AntiRoll` → `"anti_roll"`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationCategory {
    Springs,
    Damping,
    AntiRoll,
    RideHeight,
    Brakes,
    Tires,
    Gearing,
    Alignment,
    Aero,
    Differential,
    Engine,
}

/// Heuristic confidence level.
///
/// Serialises as snake\_case (`High` → `"high"`, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationConfidence {
    High,
    Medium,
    Low,
}

/// A single tuning adjustment (primary or alternative).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdjustmentPayload {
    /// Human-readable one-liner, e.g. `"Front spring rate 85 → 92 N/mm"`.
    pub summary: String,
    /// Tuning parameter key, e.g. `"spring_rate_front"`.
    pub parameter: String,
    /// Current value; `null` when unknown.
    pub from: Option<f64>,
    /// Recommended target value.
    pub to: f64,
    /// Smallest meaningful increment for this parameter.
    pub step: f64,
    /// Unit label, e.g. `"N/mm"`, `"mm"`, `"°"`, `"%"`.
    pub unit: String,
}

/// Full recommendation payload — the `data` object inside the WS envelope.
///
/// Core fields are defined in ADR-0002; additive fields (`corners`,
/// `needs_setup_form`, `tire_wear_max_at_emit`) are defined in ADR-0003.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecommendationPayload {
    /// ULID; unique per recommendation event.
    pub id: String,
    /// ULID; matches the active session.
    pub session_id: String,
    /// Lap on which the heuristic fired.
    pub lap_number: u32,
    /// Tuning surface area targeted by this recommendation.
    pub category: RecommendationCategory,
    /// One-line human summary.
    pub title: String,
    /// Engineer-format "Detected" line.
    pub detected: String,
    /// Engineer-format "Likely cause" line.
    pub cause: String,
    /// Primary adjustment.
    pub adjustment: AdjustmentPayload,
    /// Engineer-format "Expected outcome" line.
    pub expected_outcome: String,
    /// Heuristic confidence level.
    pub confidence: RecommendationConfidence,
    /// Zero or more caveat bullets.
    pub caveats: Vec<String>,
    /// Zero or more alternative adjustments.
    pub alternatives: Vec<AdjustmentPayload>,
    /// Driving style context used by the heuristic.
    pub driving_style_assumed: String,
    /// `true` when the heuristic fell back to a locked preset (ADR-0002).
    pub locked_fallback_used: bool,

    // ADR-0003 additive fields ------------------------------------------------
    /// Corner labels (e.g. `"T1"`) where the symptom was observed.
    pub corners: Vec<String>,
    /// `true` when the overlay should prompt for a fresh setup form.
    pub needs_setup_form: bool,
    /// Highest per-tyre wear fraction `[0.0, 1.0]` at emit time.
    pub tire_wear_max_at_emit: f32,
}

/// Returns a fully-populated canonical stub used by `POST /admin/test/recommendation`.
///
/// The stub demonstrates every field in the schema, making it suitable for
/// overlay renderer development and integration tests.
pub fn stub_recommendation() -> RecommendationPayload {
    RecommendationPayload {
        id: "01HQ7K8YV3STUB000000001A".to_string(),
        session_id: "01HQ7K8YV3STUB000000000S".to_string(),
        lap_number: 3,
        category: RecommendationCategory::Springs,
        title: "Front bottoming out".to_string(),
        detected: "Front suspension >95% travel on 3 of 4 corners (T1, T3, T7).".to_string(),
        cause: "Insufficient front spring rate / ride height for downforce + load.".to_string(),
        adjustment: AdjustmentPayload {
            summary: "Front spring rate 85 \u{2192} 92 N/mm".to_string(),
            parameter: "spring_rate_front".to_string(),
            from: Some(85.0),
            to: 92.0,
            step: 1.0,
            unit: "N/mm".to_string(),
        },
        expected_outcome:
            "Eliminates bottoming on T1/T3; slight loss of mechanical grip mid-corner.".to_string(),
        confidence: RecommendationConfidence::High,
        caveats: vec![
            "Assumes smooth driving style".to_string(),
            "Re-check after 3 clean laps".to_string(),
            "If Race Springs not installed, raise ride height +2mm instead".to_string(),
        ],
        alternatives: vec![AdjustmentPayload {
            summary: "Ride height F +2mm".to_string(),
            parameter: "ride_height_front".to_string(),
            from: Some(110.0),
            to: 112.0,
            step: 1.0,
            unit: "mm".to_string(),
        }],
        driving_style_assumed: "smooth".to_string(),
        locked_fallback_used: false,
        corners: vec!["T1".to_string(), "T3".to_string(), "T7".to_string()],
        needs_setup_form: false,
        tire_wear_max_at_emit: 0.15,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    // ── Category enum ────────────────────────────────────────────────────────

    #[test]
    fn category_serialises_to_snake_case() {
        assert_eq!(
            serde_json::to_value(RecommendationCategory::AntiRoll).unwrap(),
            json!("anti_roll")
        );
        assert_eq!(
            serde_json::to_value(RecommendationCategory::RideHeight).unwrap(),
            json!("ride_height")
        );
        assert_eq!(
            serde_json::to_value(RecommendationCategory::Differential).unwrap(),
            json!("differential")
        );
        assert_eq!(
            serde_json::to_value(RecommendationCategory::Engine).unwrap(),
            json!("engine")
        );
    }

    #[test]
    fn category_roundtrips_all_variants() {
        let variants = [
            RecommendationCategory::Springs,
            RecommendationCategory::Damping,
            RecommendationCategory::AntiRoll,
            RecommendationCategory::RideHeight,
            RecommendationCategory::Brakes,
            RecommendationCategory::Tires,
            RecommendationCategory::Gearing,
            RecommendationCategory::Alignment,
            RecommendationCategory::Aero,
            RecommendationCategory::Differential,
            RecommendationCategory::Engine,
        ];
        for variant in variants {
            let serialised = serde_json::to_string(&variant).expect("serialise category");
            let deserialised: RecommendationCategory =
                serde_json::from_str(&serialised).expect("deserialise category");
            assert_eq!(variant, deserialised);
        }
    }

    // ── Confidence enum ──────────────────────────────────────────────────────

    #[test]
    fn confidence_serialises_to_lowercase() {
        assert_eq!(
            serde_json::to_value(RecommendationConfidence::High).unwrap(),
            json!("high")
        );
        assert_eq!(
            serde_json::to_value(RecommendationConfidence::Medium).unwrap(),
            json!("medium")
        );
        assert_eq!(
            serde_json::to_value(RecommendationConfidence::Low).unwrap(),
            json!("low")
        );
    }

    // ── AdjustmentPayload ────────────────────────────────────────────────────

    #[test]
    fn adjustment_from_null_serialises_as_json_null() {
        let adj = AdjustmentPayload {
            summary: "test".to_string(),
            parameter: "spring_rate_front".to_string(),
            from: None,
            to: 92.0,
            step: 1.0,
            unit: "N/mm".to_string(),
        };
        let value = serde_json::to_value(&adj).unwrap();
        assert_eq!(value["from"], json!(null));
        assert_eq!(value["to"], json!(92.0));
        assert_eq!(value["step"], json!(1.0));
        assert_eq!(value["unit"], json!("N/mm"));
        assert_eq!(value["parameter"], json!("spring_rate_front"));
    }

    #[test]
    fn adjustment_from_some_serialises_as_number() {
        let adj = AdjustmentPayload {
            summary: "test".to_string(),
            parameter: "spring_rate_front".to_string(),
            from: Some(85.0),
            to: 92.0,
            step: 1.0,
            unit: "N/mm".to_string(),
        };
        let value = serde_json::to_value(&adj).unwrap();
        assert_eq!(value["from"], json!(85.0));
    }

    // ── RecommendationPayload ────────────────────────────────────────────────

    #[test]
    fn stub_recommendation_roundtrips_through_json() {
        let original = stub_recommendation();
        let json_str = serde_json::to_string(&original).expect("serialise stub");
        let recovered: RecommendationPayload =
            serde_json::from_str(&json_str).expect("deserialise stub");
        assert_eq!(original, recovered);
    }

    #[test]
    fn stub_recommendation_json_shape_matches_adr_0003() {
        let stub = stub_recommendation();
        let value = serde_json::to_value(&stub).expect("serialise stub");

        // Envelope-level data fields required by ADR-0003
        assert!(value["id"].is_string(), "id must be string");
        assert!(value["session_id"].is_string(), "session_id must be string");
        assert!(value["lap_number"].is_number(), "lap_number must be number");

        // category must be a snake_case string
        assert_eq!(value["category"], json!("springs"));

        // confidence must be one of the enum literals
        assert_eq!(value["confidence"], json!("high"));

        // adjustment object shape
        let adj = &value["adjustment"];
        assert!(adj.is_object(), "adjustment must be object");
        assert!(adj["summary"].is_string());
        assert!(adj["parameter"].is_string());
        assert!(
            adj["from"].is_number(),
            "from must be number (not null in stub)"
        );
        assert!(adj["to"].is_number());
        assert!(adj["step"].is_number());
        assert!(adj["unit"].is_string());

        // arrays
        assert!(value["caveats"].is_array());
        assert!(value["alternatives"].is_array());
        assert!(!value["alternatives"].as_array().unwrap().is_empty());

        // ADR-0003 additive fields
        let corners = value["corners"].as_array().expect("corners must be array");
        assert!(!corners.is_empty(), "stub corners must be non-empty");
        assert!(
            corners.iter().all(|c| c.is_string()),
            "all corners must be strings"
        );

        assert!(
            value["needs_setup_form"].is_boolean(),
            "needs_setup_form must be bool"
        );
        assert!(
            value["tire_wear_max_at_emit"].is_number(),
            "tire_wear_max_at_emit must be number"
        );

        // locked_fallback_used present (ADR-0002 field, carried forward)
        assert!(value["locked_fallback_used"].is_boolean());
    }

    #[test]
    fn full_ws_envelope_shape_with_recommendation() {
        use serde_json::json;

        let stub = stub_recommendation();
        let data = serde_json::to_value(&stub).expect("serialise stub");

        // Simulate the EventMessage envelope (type, schema_version, t_ms, data)
        let envelope = json!({
            "type": "recommendation",
            "schema_version": 1,
            "t_ms": 1738012345678_u64,
            "data": data,
        });

        assert_eq!(envelope["type"], json!("recommendation"));
        assert_eq!(envelope["schema_version"], json!(1));
        assert!(envelope["t_ms"].is_number());
        assert!(envelope["data"].is_object());
        assert_eq!(envelope["data"]["category"], json!("springs"));
        assert_eq!(envelope["data"]["confidence"], json!("high"));
        assert!(envelope["data"]["corners"].is_array());
        assert!(envelope["data"]["needs_setup_form"].is_boolean());
        assert!(envelope["data"]["tire_wear_max_at_emit"].is_number());
    }
}
