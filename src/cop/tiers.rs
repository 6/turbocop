use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Stability tier for a cop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Stable,
    Preview,
}

/// Schema for resources/tiers.json.
#[derive(Deserialize)]
struct TiersFile {
    #[allow(dead_code)]
    schema: u32,
    default_tier: Tier,
    overrides: HashMap<String, Tier>,
}

/// Compiled tier map: cop name → tier.
pub struct TierMap {
    default_tier: Tier,
    overrides: HashMap<String, Tier>,
}

impl TierMap {
    /// Load from the embedded resources/tiers.json, or from `TURBOCOP_TIERS_FILE` if set.
    pub fn load() -> Self {
        let json = match std::env::var("TURBOCOP_TIERS_FILE") {
            Ok(path) => std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("TURBOCOP_TIERS_FILE={path}: {e}")),
            Err(_) => include_str!("../resources/tiers.json").to_string(),
        };
        let data: TiersFile = serde_json::from_str(&json).expect("tiers JSON should be valid");
        Self {
            default_tier: data.default_tier,
            overrides: data.overrides,
        }
    }

    /// Get the tier for a cop by name.
    pub fn tier_for(&self, cop_name: &str) -> Tier {
        self.overrides
            .get(cop_name)
            .copied()
            .unwrap_or(self.default_tier)
    }
}

/// Tracks cops that were enabled by config but not run.
#[derive(Default, Debug, Clone, Serialize)]
pub struct SkipSummary {
    /// Implemented cops at preview tier, skipped because `--preview` was not set.
    pub preview_gated: Vec<String>,
    /// Cops in the vendor baseline but not implemented in turbocop's registry.
    pub unimplemented: Vec<String>,
    /// Cops not in the vendor baseline at all (unknown/custom cops).
    pub outside_baseline: Vec<String>,
}

impl SkipSummary {
    pub fn total(&self) -> usize {
        self.preview_gated.len() + self.unimplemented.len() + self.outside_baseline.len()
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_tiers_from_embedded_json() {
        let map = TierMap::load();
        assert_eq!(map.default_tier, Tier::Preview);
        assert!(!map.overrides.is_empty());
    }

    #[test]
    fn tier_for_stable_cop() {
        let map = TierMap::load();
        // Pick any cop from stable overrides — test is resilient to oracle updates
        let stable_cop = map
            .overrides
            .iter()
            .find(|(_, tier)| **tier == Tier::Stable)
            .expect("tiers.json should have at least one stable cop");
        assert_eq!(map.tier_for(stable_cop.0), Tier::Stable);
    }

    #[test]
    fn tier_for_preview_cop() {
        let map = TierMap::load();
        // Cops not in overrides default to preview
        assert_eq!(map.tier_for("Style/StringLiterals"), Tier::Preview);
    }

    #[test]
    fn tier_for_unknown_cop_uses_default() {
        let map = TierMap::load();
        // New/unknown cops default to preview (must earn stable via corpus)
        assert_eq!(map.tier_for("Custom/MyCop"), Tier::Preview);
    }

    #[test]
    fn skip_summary_default_is_empty() {
        let s = SkipSummary::default();
        assert!(s.is_empty());
        assert_eq!(s.total(), 0);
    }

    #[test]
    fn skip_summary_counts() {
        let s = SkipSummary {
            preview_gated: vec!["A/B".into(), "C/D".into()],
            unimplemented: vec!["E/F".into()],
            outside_baseline: vec!["G/H".into(), "I/J".into(), "K/L".into()],
        };
        assert_eq!(s.total(), 6);
        assert!(!s.is_empty());
    }
}
