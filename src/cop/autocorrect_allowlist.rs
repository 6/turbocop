use std::collections::HashSet;

static ALLOWLIST_JSON: &str = include_str!("../resources/autocorrect_safe_allowlist.json");

/// Set of cop names whose autocorrect output has been verified safe.
/// Used to restrict `-a` (safe autocorrect) to only allowlisted cops.
/// `-A` (all autocorrect) bypasses this check.
pub struct AutocorrectAllowlist {
    cops: HashSet<String>,
}

impl AutocorrectAllowlist {
    pub fn load() -> Self {
        let list: Vec<String> =
            serde_json::from_str(ALLOWLIST_JSON).expect("autocorrect_safe_allowlist.json is valid");
        Self {
            cops: list.into_iter().collect(),
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.cops.contains(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_loads_and_contains() {
        let al = AutocorrectAllowlist::load();
        assert!(al.contains("Layout/TrailingWhitespace"));
        assert!(al.contains("Style/FrozenStringLiteralComment"));
        assert!(!al.contains("Style/FakeNonexistentCop"));
    }
}
