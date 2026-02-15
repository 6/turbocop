use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct NamedSubject;

/// Flags usage of bare `subject` inside examples/hooks when it should be named.
///
/// EnforcedStyle:
/// - `always` (default): flag every bare `subject` reference
/// - `named_only`: only flag when the file contains a named subject declaration
impl Cop for NamedSubject {
    fn name(&self) -> &'static str {
        "RSpec/NamedSubject"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "always");
        let bytes = source.as_bytes();

        // For `named_only` style, check if the file has any named subject declarations
        // (e.g., `subject(:foo)` or `subject(:foo) { ... }`).
        if style == "named_only" && !file_has_named_subject(bytes) {
            return Vec::new();
        }

        // Walk the AST to find bare `subject` references
        let mut finder = BareSubjectFinder {
            source,
            cop: self,
            diags: Vec::new(),
        };
        finder.visit(&parse_result.node());
        finder.diags
    }
}

/// Check if the source bytes contain a named subject declaration pattern.
/// Looks for `subject(:` which indicates `subject(:name)` or `subject(:name) { ... }`.
fn file_has_named_subject(bytes: &[u8]) -> bool {
    bytes.windows(9).any(|w| w == b"subject(:")
}

struct BareSubjectFinder<'a> {
    source: &'a SourceFile,
    cop: &'a NamedSubject,
    diags: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for BareSubjectFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"subject"
            && node.receiver().is_none()
            && node.block().is_none()
            && node.arguments().is_none()
        {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diags.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Name your test subject if you need to reference it explicitly.".to_string(),
            ));
        }

        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NamedSubject, "cops/rspec/named_subject");

    #[test]
    fn named_only_style_skips_without_named_subject() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("named_only".into()),
            )]),
            ..CopConfig::default()
        };
        // File with bare `subject` but no named subject declaration
        let source = b"describe Foo do\n  it 'works' do\n    expect(subject).to be_valid\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&NamedSubject, source, config);
        assert!(diags.is_empty(), "named_only should not flag without named subject");
    }
}
