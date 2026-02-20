use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Convention,
    Warning,
    Error,
    Fatal,
}

impl Severity {
    pub fn letter(&self) -> char {
        match self {
            Severity::Convention => 'C',
            Severity::Warning => 'W',
            Severity::Error => 'E',
            Severity::Fatal => 'F',
        }
    }

    pub fn from_str(s: &str) -> Option<Severity> {
        match s.to_lowercase().as_str() {
            "convention" | "c" => Some(Severity::Convention),
            "warning" | "w" => Some(Severity::Warning),
            "error" | "e" => Some(Severity::Error),
            "fatal" | "f" => Some(Severity::Fatal),
            _ => None,
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.letter())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// 1-indexed line number
    pub line: usize,
    /// 0-indexed column (byte offset within the line)
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub path: String,
    pub location: Location,
    pub severity: Severity,
    pub cop_name: String,
    pub message: String,
}

impl Diagnostic {
    pub fn sort_key(&self) -> (&str, usize, usize) {
        (&self.path, self.location.line, self.location.column)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}: {}: {}",
            self.path,
            self.location.line,
            self.location.column,
            self.severity,
            self.cop_name,
            self.message,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_letters() {
        assert_eq!(Severity::Convention.letter(), 'C');
        assert_eq!(Severity::Warning.letter(), 'W');
        assert_eq!(Severity::Error.letter(), 'E');
        assert_eq!(Severity::Fatal.letter(), 'F');
    }

    #[test]
    fn severity_display() {
        assert_eq!(format!("{}", Severity::Convention), "C");
        assert_eq!(format!("{}", Severity::Fatal), "F");
    }

    #[test]
    fn severity_from_str() {
        assert_eq!(Severity::from_str("convention"), Some(Severity::Convention));
        assert_eq!(Severity::from_str("Warning"), Some(Severity::Warning));
        assert_eq!(Severity::from_str("ERROR"), Some(Severity::Error));
        assert_eq!(Severity::from_str("fatal"), Some(Severity::Fatal));
        assert_eq!(Severity::from_str("unknown"), None);
        // Single-letter codes (RuboCop compat)
        assert_eq!(Severity::from_str("c"), Some(Severity::Convention));
        assert_eq!(Severity::from_str("W"), Some(Severity::Warning));
        assert_eq!(Severity::from_str("E"), Some(Severity::Error));
        assert_eq!(Severity::from_str("F"), Some(Severity::Fatal));
    }

    #[test]
    fn severity_ordering() {
        assert!(Severity::Convention < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Fatal);
    }

    #[test]
    fn diagnostic_display() {
        let d = Diagnostic {
            path: "foo.rb".to_string(),
            location: Location { line: 3, column: 5 },
            severity: Severity::Convention,
            cop_name: "Style/Foo".to_string(),
            message: "bad style".to_string(),
        };
        assert_eq!(format!("{d}"), "foo.rb:3:5: C: Style/Foo: bad style");
    }

    #[test]
    fn diagnostic_sort_key() {
        let d1 = Diagnostic {
            path: "a.rb".to_string(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Convention,
            cop_name: "X".to_string(),
            message: "m".to_string(),
        };
        let d2 = Diagnostic {
            path: "a.rb".to_string(),
            location: Location { line: 2, column: 0 },
            severity: Severity::Convention,
            cop_name: "X".to_string(),
            message: "m".to_string(),
        };
        let d3 = Diagnostic {
            path: "b.rb".to_string(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Convention,
            cop_name: "X".to_string(),
            message: "m".to_string(),
        };
        assert!(d1.sort_key() < d2.sort_key());
        assert!(d2.sort_key() < d3.sort_key());
    }

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        fn severity_strategy() -> impl Strategy<Value = Severity> {
            prop::sample::select(vec![
                Severity::Convention,
                Severity::Warning,
                Severity::Error,
                Severity::Fatal,
            ])
        }

        fn diagnostic_strategy() -> impl Strategy<Value = Diagnostic> {
            (
                "[a-z]{1,5}\\.rb",
                1usize..500,
                0usize..200,
                severity_strategy(),
                "[A-Z][a-z]+/[A-Z][a-z]+",
                "[a-z ]{1,30}",
            )
                .prop_map(|(path, line, column, severity, cop_name, message)| {
                    Diagnostic {
                        path,
                        location: Location { line, column },
                        severity,
                        cop_name,
                        message,
                    }
                })
        }

        proptest! {
            #[test]
            fn sort_key_ordering_is_transitive(
                a in diagnostic_strategy(),
                b in diagnostic_strategy(),
                c in diagnostic_strategy(),
            ) {
                if a.sort_key() < b.sort_key() && b.sort_key() < c.sort_key() {
                    prop_assert!(a.sort_key() < c.sort_key());
                }
            }

            #[test]
            fn sort_produces_correct_order(mut diagnostics in prop::collection::vec(diagnostic_strategy(), 0..20)) {
                diagnostics.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
                for pair in diagnostics.windows(2) {
                    prop_assert!(pair[0].sort_key() <= pair[1].sort_key());
                }
            }

            #[test]
            fn display_contains_all_fields(d in diagnostic_strategy()) {
                let output = format!("{d}");
                prop_assert!(output.contains(&d.path));
                prop_assert!(output.contains(&d.location.line.to_string()));
                prop_assert!(output.contains(&d.location.column.to_string()));
                prop_assert!(output.contains(&d.severity.letter().to_string()));
                prop_assert!(output.contains(&d.cop_name));
                prop_assert!(output.contains(&d.message));
            }

            #[test]
            fn severity_from_str_roundtrip(sev in severity_strategy()) {
                let name = match sev {
                    Severity::Convention => "convention",
                    Severity::Warning => "warning",
                    Severity::Error => "error",
                    Severity::Fatal => "fatal",
                };
                prop_assert_eq!(Severity::from_str(name), Some(sev));
            }

            #[test]
            fn severity_from_str_rejects_random(s in "[a-z]{6,20}") {
                // Random strings of 6+ lowercase chars won't match any severity name
                let valid = ["convention", "warning", "error", "fatal"];
                if !valid.contains(&s.as_str()) {
                    prop_assert_eq!(Severity::from_str(&s), None);
                }
            }
        }
    }
}
