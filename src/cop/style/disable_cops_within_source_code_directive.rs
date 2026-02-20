use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DisableCopsWithinSourceCodeDirective;

impl Cop for DisableCopsWithinSourceCodeDirective {
    fn name(&self) -> &'static str {
        "Style/DisableCopsWithinSourceCodeDirective"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig, diagnostics: &mut Vec<Diagnostic>, _corrections: Option<&mut Vec<crate::correction::Correction>>) {
        let allowed_cops = config.get_string_array("AllowedCops").unwrap_or_default();

        for (i, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Look for rubocop:disable or rubocop:enable directives
            for directive in &["# rubocop:disable ", "# rubocop:enable ", "# rubocop:todo "] {
                if let Some(pos) = line_str.find(directive) {
                    let directive_str = &line_str[pos..];

                    // Extract cop names from the directive
                    let cops_part = &directive_str[directive.len()..].trim();

                    if !allowed_cops.is_empty() {
                        // Check if all cops in the directive are allowed
                        let cop_names: Vec<&str> = cops_part
                            .split(',')
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .collect();

                        let disallowed: Vec<&&str> = cop_names
                            .iter()
                            .filter(|c| !allowed_cops.iter().any(|a| a == *c))
                            .collect();

                        if disallowed.is_empty() {
                            continue;
                        }

                        for cop in disallowed {
                            diagnostics.push(self.diagnostic(
                                source,
                                i + 1,
                                pos,
                                format!(
                                    "RuboCop disable/enable directives for `{}` are not permitted.",
                                    cop
                                ),
                            ));
                        }
                    } else {
                        diagnostics.push(self.diagnostic(
                            source,
                            i + 1,
                            pos,
                            "RuboCop disable/enable directives are not permitted.".to_string(),
                        ));
                    }

                    break; // Only report once per line
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DisableCopsWithinSourceCodeDirective,
        "cops/style/disable_cops_within_source_code_directive"
    );
}
