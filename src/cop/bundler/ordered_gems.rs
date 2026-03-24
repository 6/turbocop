use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Extended corpus FN investigation (2026-03-19):
/// - 2 FN from multi-line gem declarations (git:, glob: continuation lines
///   were resetting prev_gem). Fixed by skipping continuation lines.
/// - 8 FN from inline conditional gem calls (e.g., `if cond; gem 'x' else gem 'y', path: 'z' end`).
///   Fixed by scanning for `gem 'name'` patterns anywhere on the line (not just at start),
///   with comment stripping and word-boundary checks.
///
/// ## Corpus investigation (2026-03-20)
///
/// Corpus oracle reported FP=1, FN=0.
///
/// FP=1: `gsub_file 'Gemfile', /gem 'pg'.*/, ''` — the regex literal `/gem 'pg'.*/`
/// contained `gem 'pg'` preceded by `/`, which passed the word boundary check.
/// Fixed by adding `b'/'` to the boundary exclusion set in `extract_literal_gem_name`.
/// In Gemfiles, `gem` method calls are never preceded by `/`; this only occurs in
/// regex literals or path strings.
pub struct OrderedGems;

/// A gem entry within a group, tracking its byte range for autocorrect.
struct GemEntry {
    #[allow(dead_code)]
    name: String,
    sort_key: String,
    /// Byte offset of the start of the line (inclusive).
    line_start: usize,
    /// Byte offset past the end of the line including newline (exclusive).
    line_end: usize,
}

impl Cop for OrderedGems {
    fn name(&self) -> &'static str {
        "Bundler/OrderedGems"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
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
        let mut prev_gem: Option<(String, String)> = None; // (original_name, sort_key)
        let mut in_block_comment = false;

        // Track gem groups for autocorrect: groups of contiguous gem lines
        let mut gem_group: Vec<GemEntry> = Vec::new();

        // Compute line byte ranges
        let mut line_offsets: Vec<(usize, usize)> = Vec::new(); // (start, end) for each line
        {
            let mut offset = 0;
            for line in source.lines() {
                let start = offset;
                offset += line.len();
                // Skip past newline
                if offset < bytes.len() && bytes[offset] == b'\n' {
                    offset += 1;
                }
                line_offsets.push((start, offset));
            }
        }

        let flush_group = |gem_group: &mut Vec<GemEntry>,
                           corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
                           bytes: &[u8]| {
            if gem_group.len() >= 2 {
                if let Some(corr) = corrections {
                    // Check if the group is already sorted
                    let is_sorted = gem_group.windows(2).all(|w| w[0].sort_key <= w[1].sort_key);
                    if !is_sorted {
                        let group_start = gem_group.first().unwrap().line_start;
                        let group_end = gem_group.last().unwrap().line_end;
                        // Sort by sort_key using index array
                        let mut indices: Vec<usize> = (0..gem_group.len()).collect();
                        indices.sort_by(|&a, &b| gem_group[a].sort_key.cmp(&gem_group[b].sort_key));
                        let sorted: Vec<&[u8]> = indices
                            .iter()
                            .map(|&i| &bytes[gem_group[i].line_start..gem_group[i].line_end])
                            .collect();
                        let replacement: Vec<u8> = sorted.concat();
                        corr.push(crate::correction::Correction {
                            start: group_start,
                            end: group_end,
                            replacement: String::from_utf8_lossy(&replacement).to_string(),
                            cop_name: "Bundler/OrderedGems",
                            cop_index: 0,
                        });
                    }
                }
            }
            gem_group.clear();
        };

        for (i, line) in source.lines().enumerate() {
            let line_str = std::str::from_utf8(line).unwrap_or("");
            let trimmed = line_str.trim();
            let line_num = i + 1;
            let (line_start, line_end) = line_offsets[i];

            if in_block_comment {
                if trimmed.starts_with("=end") {
                    in_block_comment = false;
                    prev_gem = None;
                    flush_group(&mut gem_group, &mut corrections, bytes);
                }
                continue;
            }

            if trimmed.starts_with("=begin") {
                in_block_comment = true;
                prev_gem = None;
                flush_group(&mut gem_group, &mut corrections, bytes);
                continue;
            }

            // Blank lines reset the ordering group
            if trimmed.is_empty() {
                flush_group(&mut gem_group, &mut corrections, bytes);
                prev_gem = None;
                continue;
            }

            // Comments may reset the ordering group
            if trimmed.starts_with('#') {
                if treat_comments_as_separators {
                    flush_group(&mut gem_group, &mut corrections, bytes);
                    prev_gem = None;
                }
                continue;
            }

            // Non-gem, non-blank, non-comment lines (like `group`, `source`, etc.)
            // also reset the ordering group
            if let Some(gem_name) = extract_literal_gem_name(line_str) {
                let sort_key = make_sort_key(gem_name, consider_punctuation);
                let col = line_str.len() - line_str.trim_start().len();

                if let Some((ref prev_name, ref prev_key)) = prev_gem {
                    if sort_key < *prev_key {
                        let mut diag = self.diagnostic(
                            source,
                            line_num,
                            col,
                            format!(
                                "Gems should be sorted in an alphabetical order within their section of the Gemfile. Gem `{}` should appear before `{}`.",
                                gem_name, prev_name
                            ),
                        );
                        if corrections.is_some() {
                            diag.corrected = true;
                        }
                        diagnostics.push(diag);
                    }
                }

                gem_group.push(GemEntry {
                    name: gem_name.to_string(),
                    sort_key: sort_key.clone(),
                    line_start,
                    line_end,
                });

                prev_gem = Some((gem_name.to_string(), sort_key));
            } else if is_continuation_line(trimmed) {
                // Continuation lines of multi-line gem declarations — extend the last
                // gem entry's range to include this line
                if let Some(last) = gem_group.last_mut() {
                    last.line_end = line_end;
                }
            } else {
                // Non-gem declaration resets the group (group, source, platforms, etc.)
                flush_group(&mut gem_group, &mut corrections, bytes);
                prev_gem = None;
            }
        }

        // Flush remaining group
        flush_group(&mut gem_group, &mut corrections, bytes);
    }
}

/// Check if a trimmed line looks like a continuation of a multi-line gem declaration.
/// Continuation lines are typically keyword arguments (git:, path:, glob:, require:),
/// version strings ('0.1.1'), or other argument content that follows a trailing comma.
fn is_continuation_line(trimmed: &str) -> bool {
    // Starts with a quoted string (version constraint like '0.1.1')
    if trimmed.starts_with('\'') || trimmed.starts_with('"') {
        return true;
    }
    // Starts with a symbol like :development
    if trimmed.starts_with(':') {
        return true;
    }
    // Keyword argument (e.g., git:, path:, glob:, require:, platforms:, group:)
    // These look like `word:` possibly followed by a space
    if let Some(colon_pos) = trimmed.find(':') {
        let before_colon = &trimmed[..colon_pos];
        if !before_colon.is_empty()
            && before_colon
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return true;
        }
    }
    false
}

/// Extract the gem name from literal first-argument forms:
/// - `gem 'foo'`
/// - `gem "foo"`
/// - `gem('foo')`
///
/// Also finds gem calls mid-line (e.g., `if cond; gem 'foo' else gem 'foo', path: 'x' end`).
/// Lines like `gem ENV['FOO'] || 'foo'` are intentionally ignored.
fn extract_literal_gem_name(line: &str) -> Option<&str> {
    // Strip the comment portion of the line (anything after an unquoted #)
    let code_part = strip_comment(line);

    // Scan for `gem` followed by whitespace or `(`, then a quoted string
    let bytes = code_part.as_bytes();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        // Find next occurrence of "gem"
        if let Some(pos) = code_part[i..].find("gem") {
            let abs_pos = i + pos;
            // Check word boundary before "gem": must be start of string or non-alphanumeric
            if abs_pos > 0 {
                let prev = bytes[abs_pos - 1];
                if prev.is_ascii_alphanumeric() || prev == b'_' || prev == b'/' {
                    i = abs_pos + 3;
                    continue;
                }
            }
            // Check what follows "gem"
            let after_gem = &code_part[abs_pos + 3..];
            if let Some(name) = extract_gem_arg(after_gem) {
                return Some(name);
            }
            i = abs_pos + 3;
        } else {
            break;
        }
    }
    None
}

/// Extract the gem name from the portion after "gem" (whitespace/paren then quoted string).
fn extract_gem_arg(after_gem: &str) -> Option<&str> {
    let first = after_gem.chars().next()?;
    if !first.is_whitespace() && first != '(' {
        return None;
    }

    let mut rest = after_gem.trim_start();
    if let Some(after_paren) = rest.strip_prefix('(') {
        rest = after_paren.trim_start();
    }

    let quote = rest.as_bytes().first().copied()?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }

    let content = &rest[1..];
    let end = content.find(quote as char)?;
    Some(&content[..end])
}

/// Strip the comment portion from a line of Ruby code.
/// Returns the code portion before any unquoted `#`.
fn strip_comment(line: &str) -> &str {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        if in_single_quote {
            if ch == b'\'' {
                in_single_quote = false;
            }
            // No escape handling in single-quoted strings for this purpose
        } else if in_double_quote {
            if ch == b'\\' {
                i += 1; // skip escaped char
            } else if ch == b'"' {
                in_double_quote = false;
            }
        } else {
            match ch {
                b'\'' => in_single_quote = true,
                b'"' => in_double_quote = true,
                b'#' => return &line[..i],
                _ => {}
            }
        }
        i += 1;
    }
    line
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
    crate::cop_autocorrect_fixture_tests!(OrderedGems, "cops/bundler/ordered_gems");

    #[test]
    fn autocorrect_simple_swap() {
        let input = b"gem 'zoo'\ngem 'alpha'\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&OrderedGems, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"gem 'alpha'\ngem 'zoo'\n");
    }

    #[test]
    fn autocorrect_three_gems() {
        let input = b"gem 'c'\ngem 'b'\ngem 'a'\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&OrderedGems, input);
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.corrected));
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"gem 'a'\ngem 'b'\ngem 'c'\n");
    }

    #[test]
    fn autocorrect_preserves_groups() {
        let input = b"gem 'b'\ngem 'a'\n\ngem 'd'\ngem 'c'\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&OrderedGems, input);
        assert_eq!(diags.len(), 2);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"gem 'a'\ngem 'b'\n\ngem 'c'\ngem 'd'\n");
    }

    #[test]
    fn autocorrect_multiline_gem() {
        let input = b"gem 'rubocop',\n    '0.1.1'\ngem 'rspec'\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&OrderedGems, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"gem 'rspec'\ngem 'rubocop',\n    '0.1.1'\n");
    }
}
