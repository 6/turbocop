use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
            "convention" => Some(Severity::Convention),
            "warning" => Some(Severity::Warning),
            "error" => Some(Severity::Error),
            "fatal" => Some(Severity::Fatal),
            _ => None,
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.letter())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// 1-indexed line number
    pub line: usize,
    /// 0-indexed column (byte offset within the line)
    pub column: usize,
}

#[derive(Debug, Clone)]
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
}
