use std::collections::HashMap;

use super::Cop;

pub struct CopRegistry {
    cops: Vec<Box<dyn Cop>>,
    index: HashMap<&'static str, usize>,
}

impl CopRegistry {
    pub fn new() -> Self {
        Self {
            cops: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Build the default registry with all built-in cops.
    pub fn default_registry() -> Self {
        let mut registry = Self::new();
        super::layout::register_all(&mut registry);
        super::lint::register_all(&mut registry);
        super::metrics::register_all(&mut registry);
        super::naming::register_all(&mut registry);
        super::performance::register_all(&mut registry);
        super::rails::register_all(&mut registry);
        super::style::register_all(&mut registry);
        registry
    }

    pub fn register(&mut self, cop: Box<dyn Cop>) {
        let name = cop.name();
        let idx = self.cops.len();
        self.cops.push(cop);
        self.index.insert(name, idx);
    }

    pub fn cops(&self) -> &[Box<dyn Cop>] {
        &self.cops
    }

    pub fn get(&self, name: &str) -> Option<&dyn Cop> {
        self.index.get(name).map(|&idx| &*self.cops[idx])
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.cops.iter().map(|c| c.name()).collect()
    }

    pub fn len(&self) -> usize {
        self.cops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cops.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::Cop;
    use crate::diagnostic::Severity;

    struct FakeCop;

    impl Cop for FakeCop {
        fn name(&self) -> &'static str {
            "Style/Fake"
        }

        fn default_severity(&self) -> Severity {
            Severity::Warning
        }
    }

    #[test]
    fn default_registry_has_cops() {
        let reg = CopRegistry::default_registry();
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 251);
        // Spot-check cops from each department
        assert!(reg.get("Layout/TrailingWhitespace").is_some());
        assert!(reg.get("Layout/LineLength").is_some());
        assert!(reg.get("Style/FrozenStringLiteralComment").is_some());
        assert!(reg.get("Style/Tab").is_some());
        assert!(reg.get("Metrics/MethodLength").is_some());
        assert!(reg.get("Metrics/AbcSize").is_some());
        assert!(reg.get("Naming/MethodName").is_some());
        assert!(reg.get("Naming/FileName").is_some());
        // Performance department spot-checks
        assert!(reg.get("Performance/Detect").is_some());
        assert!(reg.get("Performance/FlatMap").is_some());
        assert!(reg.get("Performance/ReverseEach").is_some());
        assert!(reg.get("Performance/OpenStruct").is_some());
        assert!(reg.get("Performance/Count").is_some());
        assert!(reg.get("Style/EmptyMethod").is_some());
        assert!(reg.get("Lint/BooleanSymbol").is_some());
        assert!(reg.get("Lint/UnifiedInteger").is_some());
    }

    #[test]
    fn register_and_get() {
        let mut reg = CopRegistry::new();
        reg.register(Box::new(FakeCop));
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());

        let cop = reg.get("Style/Fake").unwrap();
        assert_eq!(cop.name(), "Style/Fake");
        assert_eq!(cop.default_severity(), Severity::Warning);
    }

    #[test]
    fn get_nonexistent() {
        let reg = CopRegistry::new();
        assert!(reg.get("Style/Nope").is_none());
    }

    #[test]
    fn names() {
        let mut reg = CopRegistry::new();
        reg.register(Box::new(FakeCop));
        assert_eq!(reg.names(), vec!["Style/Fake"]);
    }

    #[test]
    fn cops_slice() {
        let mut reg = CopRegistry::new();
        reg.register(Box::new(FakeCop));
        assert_eq!(reg.cops().len(), 1);
        assert_eq!(reg.cops()[0].name(), "Style/Fake");
    }
}
