use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle (run 22651309591) reported FP=0, FN=0. 100% conformance.
pub struct OrderedDependencies;

const DEP_METHODS: &[&str] = &[
    "add_dependency",
    "add_runtime_dependency",
    "add_development_dependency",
];

struct DepEntry {
    gem_name: String,
    sort_key: String,
    line_num: usize,
    col: usize,
    /// Byte offset of the start of the line (inclusive).
    line_start: usize,
    /// Byte offset past the end of the line including newline (exclusive).
    line_end: usize,
}

impl Cop for OrderedDependencies {
    fn name(&self) -> &'static str {
        "Gemspec/OrderedDependencies"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let treat_comments_as_separators = config.get_bool("TreatCommentsAsGroupSeparators", true);
        let consider_punctuation = config.get_bool("ConsiderPunctuation", false);

        let bytes = source.as_bytes();
        let mut current_method: Option<String> = None;
        let mut group: Vec<DepEntry> = Vec::new();

        // Compute line byte ranges
        let mut line_offsets: Vec<(usize, usize)> = Vec::new();
        {
            let mut offset = 0;
            for line in source.lines() {
                let start = offset;
                offset += line.len();
                if offset < bytes.len() && bytes[offset] == b'\n' {
                    offset += 1;
                }
                line_offsets.push((start, offset));
            }
        }

        let flush_group_autocorrect = |group: &mut Vec<DepEntry>,
                                       diagnostics: &mut Vec<Diagnostic>,
                                       source: &SourceFile,
                                       cop: &OrderedDependencies,
                                       _consider_punctuation: bool,
                                       corrections: &mut Option<
            &mut Vec<crate::correction::Correction>,
        >,
                                       bytes: &[u8]| {
            if group.len() < 2 {
                group.clear();
                return;
            }

            // Report diagnostics
            for i in 1..group.len() {
                let prev_key = &group[i - 1].sort_key;
                let curr_key = &group[i].sort_key;
                if *curr_key < *prev_key {
                    let prev_name = &group[i - 1].gem_name;
                    let curr_name = &group[i].gem_name;
                    let mut diag = cop.diagnostic(
                            source,
                            group[i].line_num,
                            group[i].col,
                            format!(
                                "Dependencies should be sorted in an alphabetical order within their section of the gemspec. Dependency `{curr_name}` should appear before `{prev_name}`."
                            ),
                        );
                    if corrections.is_some() {
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            }

            // Generate correction if needed
            if let Some(corr) = corrections {
                let is_sorted = group.windows(2).all(|w| w[0].sort_key <= w[1].sort_key);
                if !is_sorted {
                    let group_start = group.first().unwrap().line_start;
                    let group_end = group.last().unwrap().line_end;
                    let mut indices: Vec<usize> = (0..group.len()).collect();
                    indices.sort_by(|&a, &b| group[a].sort_key.cmp(&group[b].sort_key));
                    let sorted: Vec<&[u8]> = indices
                        .iter()
                        .map(|&i| &bytes[group[i].line_start..group[i].line_end])
                        .collect();
                    let replacement: Vec<u8> = sorted.concat();
                    corr.push(crate::correction::Correction {
                        start: group_start,
                        end: group_end,
                        replacement: String::from_utf8_lossy(&replacement).to_string(),
                        cop_name: "Gemspec/OrderedDependencies",
                        cop_index: 0,
                    });
                }
            }

            group.clear();
        };

        let lines: Vec<&[u8]> = source.lines().collect();
        for (line_idx, line) in lines.iter().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => {
                    flush_group_autocorrect(
                        &mut group,
                        diagnostics,
                        source,
                        self,
                        consider_punctuation,
                        &mut corrections,
                        bytes,
                    );
                    current_method = None;
                    continue;
                }
            };
            let trimmed = line_str.trim();
            let (line_start, line_end) = line_offsets[line_idx];

            // Blank lines act as group separators
            if trimmed.is_empty() {
                flush_group_autocorrect(
                    &mut group,
                    diagnostics,
                    source,
                    self,
                    consider_punctuation,
                    &mut corrections,
                    bytes,
                );
                current_method = None;
                continue;
            }

            // Check if this is a comment line
            if trimmed.starts_with('#') {
                if treat_comments_as_separators {
                    flush_group_autocorrect(
                        &mut group,
                        diagnostics,
                        source,
                        self,
                        consider_punctuation,
                        &mut corrections,
                        bytes,
                    );
                    current_method = None;
                }
                continue;
            }

            // Check if this is a dependency call
            let mut found_dep = false;
            for &method in DEP_METHODS {
                let dot_method = format!(".{method}");
                if let Some(pos) = line_str.find(&dot_method) {
                    let after = &line_str[pos + dot_method.len()..];
                    if let Some(gem_name) = extract_gem_name(after) {
                        if current_method.as_deref() != Some(method) {
                            // Different dependency type, flush previous group
                            flush_group_autocorrect(
                                &mut group,
                                diagnostics,
                                source,
                                self,
                                consider_punctuation,
                                &mut corrections,
                                bytes,
                            );
                            current_method = Some(method.to_string());
                        }
                        let sk = sort_key(&gem_name, consider_punctuation);
                        group.push(DepEntry {
                            gem_name,
                            sort_key: sk,
                            line_num: line_idx + 1,
                            col: pos + 1, // after the dot
                            line_start,
                            line_end,
                        });
                        found_dep = true;
                    }
                    break;
                }
            }

            if !found_dep && !trimmed.is_empty() {
                flush_group_autocorrect(
                    &mut group,
                    diagnostics,
                    source,
                    self,
                    consider_punctuation,
                    &mut corrections,
                    bytes,
                );
                current_method = None;
            }
        }

        // Flush remaining group
        flush_group_autocorrect(
            &mut group,
            diagnostics,
            source,
            self,
            consider_punctuation,
            &mut corrections,
            bytes,
        );
    }
}

fn sort_key(name: &str, consider_punctuation: bool) -> String {
    if consider_punctuation {
        name.to_lowercase()
    } else {
        // Remove all hyphens and underscores, then lowercase.
        // This matches RuboCop's `gem_canonical_name`: `name.tr('-_', '').downcase`
        let mut result = String::with_capacity(name.len());
        for c in name.chars() {
            if c != '-' && c != '_' {
                result.push(c.to_ascii_lowercase());
            }
        }
        result
    }
}

/// Extract the gem name from the arguments after a dependency method call.
/// Returns `None` when the argument uses `.freeze` (e.g., `%q<name>.freeze` or
/// `"name".freeze`), matching RuboCop's AST pattern which requires a bare `(str _)`.
fn extract_gem_name(after_method: &str) -> Option<String> {
    let s = after_method.trim_start();
    let s = if let Some(stripped) = s.strip_prefix('(') {
        stripped.trim_start()
    } else {
        s
    };

    if s.starts_with('\'') || s.starts_with('"') {
        let quote = s.as_bytes()[0];
        let rest = &s[1..];
        let end = rest.find(|c: char| c as u8 == quote)?;
        let name = rest[..end].to_string();
        // Check for .freeze after the closing quote — RuboCop ignores these
        let after_quote = rest[end + 1..].trim_start();
        if after_quote.starts_with(".freeze") {
            return None;
        }
        Some(name)
    } else {
        let (name, consumed) = parse_percent_string_with_len(s)?;
        // Check for .freeze after the percent string — RuboCop ignores these
        let after_pct = s[consumed..].trim_start();
        if after_pct.starts_with(".freeze") {
            return None;
        }
        Some(name)
    }
}

/// Parse a Ruby percent string literal (%q<...>, %Q<...>, %q(...), etc.)
/// Returns the extracted name and the byte length consumed from `s`.
fn parse_percent_string_with_len(s: &str) -> Option<(String, usize)> {
    if !s.starts_with('%') {
        return None;
    }
    let mut offset = 1;
    // Skip optional q/Q qualifier
    if s[offset..].starts_with('q') || s[offset..].starts_with('Q') {
        offset += 1;
    }
    // Match delimiter pair
    let close = match s.as_bytes().get(offset)? {
        b'<' => '>',
        b'(' => ')',
        b'[' => ']',
        b'{' => '}',
        _ => return None,
    };
    offset += 1;
    let inner = &s[offset..];
    let end = inner.find(close)?;
    let name = inner[..end].to_string();
    offset += end + 1; // skip content + closing delimiter
    Some((name, offset))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OrderedDependencies, "cops/gemspec/ordered_dependencies");
    crate::cop_autocorrect_fixture_tests!(OrderedDependencies, "cops/gemspec/ordered_dependencies");

    #[test]
    fn autocorrect_simple_swap() {
        let input = b"Gem::Specification.new do |s|\n  s.add_dependency 'zoo'\n  s.add_dependency 'alpha'\nend\n";
        let (diags, corrections) =
            crate::testutil::run_cop_autocorrect(&OrderedDependencies, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"Gem::Specification.new do |s|\n  s.add_dependency 'alpha'\n  s.add_dependency 'zoo'\nend\n"
        );
    }

    #[test]
    fn autocorrect_preserves_group_separators() {
        let input = b"Gem::Specification.new do |s|\n  s.add_dependency 'b'\n  s.add_dependency 'a'\n\n  s.add_development_dependency 'd'\n  s.add_development_dependency 'c'\nend\n";
        let (diags, corrections) =
            crate::testutil::run_cop_autocorrect(&OrderedDependencies, input);
        assert_eq!(diags.len(), 2);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"Gem::Specification.new do |s|\n  s.add_dependency 'a'\n  s.add_dependency 'b'\n\n  s.add_development_dependency 'c'\n  s.add_development_dependency 'd'\nend\n"
        );
    }
}
