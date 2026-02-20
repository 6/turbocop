use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AddRuntimeDependency;

impl Cop for AddRuntimeDependency {
    fn name(&self) -> &'static str {
        "Gemspec/AddRuntimeDependency"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>, _corrections: Option<&mut Vec<crate::correction::Correction>>) {
        for (line_idx, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if let Some(pos) = line_str.find(".add_runtime_dependency") {
                // Column is at the dot
                diagnostics.push(self.diagnostic(
                    source,
                    line_idx + 1,
                    pos + 1, // skip the dot, point at method name
                    "Use `add_dependency` instead of `add_runtime_dependency`.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AddRuntimeDependency, "cops/gemspec/add_runtime_dependency");
}
