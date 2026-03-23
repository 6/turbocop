use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AddRuntimeDependency;

impl Cop for AddRuntimeDependency {
    fn name(&self) -> &'static str {
        "Gemspec/AddRuntimeDependency"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        for (line_idx, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            // Skip comment lines to avoid flagging commented-out code
            if line_str.trim_start().starts_with('#') {
                continue;
            }
            if let Some(pos) = line_str.find(".add_runtime_dependency") {
                let line_num = line_idx + 1;
                // Column is at the dot
                let mut diag = self.diagnostic(
                    source,
                    line_num,
                    pos + 1, // skip the dot, point at method name
                    "Use `add_dependency` instead of `add_runtime_dependency`.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    if let Some(line_start) = source.line_col_to_offset(line_num, 0) {
                        // Replace "add_runtime_dependency" with "add_dependency"
                        // pos is the position of the dot, method name starts at pos+1
                        let method_start = line_start + pos + 1;
                        let method_end = method_start + "add_runtime_dependency".len();
                        corr.push(crate::correction::Correction {
                            start: method_start,
                            end: method_end,
                            replacement: "add_dependency".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                }
                diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AddRuntimeDependency, "cops/gemspec/add_runtime_dependency");
    crate::cop_autocorrect_fixture_tests!(
        AddRuntimeDependency,
        "cops/gemspec/add_runtime_dependency"
    );

    #[test]
    fn autocorrect_replaces_method_name() {
        let input = b"Gem::Specification.new do |spec|\n  spec.add_runtime_dependency 'foo'\nend\n";
        let (diags, corrections) =
            crate::testutil::run_cop_autocorrect(&AddRuntimeDependency, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"Gem::Specification.new do |spec|\n  spec.add_dependency 'foo'\nend\n"
        );
    }

    #[test]
    fn autocorrect_multiple_occurrences() {
        let input = b"Gem::Specification.new do |s|\n  s.add_runtime_dependency 'a'\n  s.add_runtime_dependency 'b'\nend\n";
        let (diags, corrections) =
            crate::testutil::run_cop_autocorrect(&AddRuntimeDependency, input);
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.corrected));
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"Gem::Specification.new do |s|\n  s.add_dependency 'a'\n  s.add_dependency 'b'\nend\n"
        );
    }
}
