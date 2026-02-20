use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::extract_gem_name;

pub struct OrderedGems;

impl Cop for OrderedGems {
    fn name(&self) -> &'static str {
        "Bundler/OrderedGems"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig, diagnostics: &mut Vec<Diagnostic>, _corrections: Option<&mut Vec<crate::correction::Correction>>) {
        let treat_comments_as_separators =
            config.get_bool("TreatCommentsAsGroupSeparators", true);
        let consider_punctuation = config.get_bool("ConsiderPunctuation", false);

        let mut prev_gem: Option<(String, String)> = None; // (original_name, sort_key)

        for (i, line) in source.lines().enumerate() {
            let line_str = std::str::from_utf8(line).unwrap_or("");
            let trimmed = line_str.trim();
            let line_num = i + 1;

            // Blank lines reset the ordering group
            if trimmed.is_empty() {
                prev_gem = None;
                continue;
            }

            // Comments may reset the ordering group
            if trimmed.starts_with('#') {
                if treat_comments_as_separators {
                    prev_gem = None;
                }
                continue;
            }

            // Non-gem, non-blank, non-comment lines (like `group`, `source`, etc.)
            // also reset the ordering group
            if let Some(gem_name) = extract_gem_name(line_str) {
                let sort_key = make_sort_key(gem_name, consider_punctuation);

                if let Some((ref prev_name, ref prev_key)) = prev_gem {
                    if sort_key < *prev_key {
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            0,
                            format!(
                                "Gems should be sorted in an alphabetical order within their section of the Gemfile. Gem `{}` should appear before `{}`.",
                                gem_name, prev_name
                            ),
                        ));
                    }
                }

                prev_gem = Some((gem_name.to_string(), sort_key));
            } else {
                // Non-gem declaration resets the group
                prev_gem = None;
            }
        }
    }
}

/// Create a sort key for case-insensitive comparison.
/// When consider_punctuation is false, strip `-` and `_` for comparison.
fn make_sort_key(name: &str, consider_punctuation: bool) -> String {
    let lower = name.to_lowercase();
    if consider_punctuation {
        lower
    } else {
        lower.replace(['-', '_'], "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OrderedGems, "cops/bundler/ordered_gems");
}
