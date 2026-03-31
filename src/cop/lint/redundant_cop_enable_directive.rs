use std::collections::HashMap;
use std::sync::LazyLock;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Checks for `# rubocop:enable` comments that can be removed because
/// the cop was not previously disabled.
///
/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=4, FN=9.
///
/// FP=4/FN=9 came from three directive-state bugs:
/// 1. `# rubocop: enable` / `# rubocop: disable` were ignored because the
///    parser only accepted `rubocop:enable` with no whitespace after the colon.
///    That missed real redundant enables and also failed to record matching
///    disables, producing both FN and FP.
/// 2. Inline disables like `def foo # rubocop:disable MethodLength` were treated
///    as block disables, so later `# rubocop:enable MethodLength` directives
///    looked necessary even though the inline disable only applied to that line.
/// 3. Nested examples inside comments such as `#   # rubocop:enable Foo` were
///    treated as real directives, mutating the disabled set in documentation and
///    causing false positives in RuboCop's own source and similar files.
/// 4. Trailing inline enables like `end # rubocop:enable Metrics/MethodLength`
///    were treated as real enable directives. RuboCop ignores `enable` comments
///    on non-comment-only lines, so those corpus examples were false positives.
/// 5. An outer specific disable was incorrectly cleared by a nested
///    `# rubocop:disable all` / `# rubocop:enable all` pair because this cop
///    tracked `all` as a single token instead of adding one disable layer to
///    each known cop. That made valid trailing enables like
///    `#rubocop:enable Metrics/ClassLength` look redundant after an inner
///    `enable all`.
///
/// Earlier rounds already fixed trailing free-text comments and punctuation on
/// cop names. Any remaining divergence after this point would be config-aware:
/// RuboCop knows whether a target cop is disabled in project config, while this
/// cop only sees inline directives.
///
/// All known CI FP/FN locations are fixed locally, and the current rerun has
/// exact aggregate offense-count parity with RuboCop for this cop.
pub struct RedundantCopEnableDirective;

static SHORT_NAME_TO_QUALIFIED: LazyLock<HashMap<String, Vec<String>>> = LazyLock::new(|| {
    let registry = crate::cop::registry::CopRegistry::default_registry();
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    for name in registry.names() {
        if let Some((_, short)) = name.split_once('/') {
            map.entry(short.to_string())
                .or_default()
                .push(name.to_string());
        }
    }

    map
});

static ALL_KNOWN_COPS: LazyLock<Vec<String>> = LazyLock::new(|| {
    crate::cop::registry::CopRegistry::default_registry()
        .names()
        .into_iter()
        .map(|name| name.to_string())
        .collect()
});

impl Cop for RedundantCopEnableDirective {
    fn name(&self) -> &'static str {
        "Lint/RedundantCopEnableDirective"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut disabled: HashMap<String, usize> = HashMap::new();

        let mut byte_offset = 0usize;
        for (i, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => {
                    byte_offset += line.len() + 1;
                    continue;
                }
            };

            let first_comment_hash = first_comment_hash(line_str, byte_offset, code_map);
            let Some(hash_pos) = find_directive_start(line_str, first_comment_hash) else {
                byte_offset += line.len() + 1;
                continue;
            };

            if !code_map.is_not_string(byte_offset + hash_pos) {
                byte_offset += line.len() + 1;
                continue;
            }

            let Some((action, cops)) = parse_directive(&line_str[hash_pos..]) else {
                byte_offset += line.len() + 1;
                continue;
            };

            let comment_only_line = first_comment_hash
                .is_some_and(|first_hash| line_str[..first_hash].trim().is_empty());

            match action {
                "disable" | "todo" => {
                    // Inline disables apply only to the current line.
                    if comment_only_line {
                        for cop in cops {
                            if cop.eq_ignore_ascii_case("all") {
                                increment_all_known_cops(&mut disabled);
                            } else {
                                *disabled.entry(cop).or_insert(0) += 1;
                            }
                        }
                    }
                }
                "enable" => {
                    // RuboCop ignores `enable` comments on lines that also contain code.
                    if !comment_only_line {
                        byte_offset += line.len() + 1;
                        continue;
                    }

                    for cop in cops {
                        if cop == "all" {
                            if !decrement_all(&mut disabled) {
                                let col = find_cop_column(line_str, cop.as_str());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    i + 1,
                                    col,
                                    "Unnecessary enabling of all cops.".to_string(),
                                ));
                            }
                            continue;
                        }

                        let was_disabled = decrement_matching_disable(&mut disabled, cop.as_str());
                        let dept_was_disabled =
                            !was_disabled && department_disable_matches(&disabled, cop.as_str());

                        if !was_disabled && !dept_was_disabled {
                            let col = find_cop_column(line_str, cop.as_str());
                            diagnostics.push(self.diagnostic(
                                source,
                                i + 1,
                                col,
                                format!("Unnecessary enabling of {}.", cop),
                            ));
                        }
                    }
                }
                _ => {}
            }

            byte_offset += line.len() + 1;
        }
    }
}

fn find_cop_column(line: &str, cop: &str) -> usize {
    line.rfind(cop)
        .unwrap_or_else(|| line.find(cop).unwrap_or(0))
}

fn parse_directive(comment: &str) -> Option<(&str, Vec<String>)> {
    let after_hash = comment.strip_prefix('#')?.trim_start();
    let after_prefix = strip_rubocop_prefix(after_hash)?.trim_start();

    let action_end = after_prefix
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(after_prefix.len());
    let action = &after_prefix[..action_end];

    if !matches!(action, "disable" | "enable" | "todo") {
        return None;
    }

    let cops_str = after_prefix[action_end..].trim_start();
    let cops_str = cops_str.split("--").next().unwrap_or(cops_str);
    let cops: Vec<String> = cops_str.split(',').filter_map(parse_cop_token).collect();
    if cops.is_empty() {
        return None;
    }

    let action_str = match action {
        "disable" => "disable",
        "enable" => "enable",
        "todo" => "todo",
        _ => return None,
    };

    Some((action_str, cops))
}

fn parse_cop_token(raw: &str) -> Option<String> {
    let trimmed = raw.trim_start();
    let first = *trimmed.as_bytes().first()?;
    if !first.is_ascii_alphanumeric() {
        return None;
    }

    let end = trimmed
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '/' || c == '_'))
        .unwrap_or(trimmed.len());
    let token = trimmed[..end].trim_end_matches('.');
    if token.is_empty() || token.ends_with('/') {
        None
    } else {
        Some(token.to_string())
    }
}

fn decrement_matching_disable(disabled: &mut HashMap<String, usize>, cop: &str) -> bool {
    if decrement_case_insensitive(disabled, cop) {
        return true;
    }

    if let Some(short) = short_cop_name(cop) {
        if decrement_case_insensitive(disabled, short) {
            return true;
        }
        return false;
    }

    let dept_prefix = format!("{cop}/");
    let matching_department_keys: Vec<String> = disabled
        .iter()
        .filter(|(name, count)| {
            **count > 0 && starts_with_case_insensitive(name.as_str(), dept_prefix.as_str())
        })
        .map(|(name, _)| name.clone())
        .collect();
    if !matching_department_keys.is_empty() {
        for key in matching_department_keys {
            decrement_exact(disabled, key.as_str());
        }
        return true;
    }

    let matching_short_key = disabled
        .iter()
        .find(|(name, count)| {
            **count > 0
                && short_cop_name(name.as_str())
                    .is_some_and(|short| short.eq_ignore_ascii_case(cop))
        })
        .map(|(name, _)| name.clone());
    if let Some(key) = matching_short_key {
        decrement_exact(disabled, key.as_str());
        return true;
    }

    false
}

fn increment_all_known_cops(disabled: &mut HashMap<String, usize>) {
    for cop in ALL_KNOWN_COPS.iter() {
        *disabled.entry(cop.clone()).or_insert(0) += 1;
    }
}

fn decrement_all(disabled: &mut HashMap<String, usize>) -> bool {
    let keys: Vec<String> = disabled
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(name, _)| name.clone())
        .collect();
    if keys.is_empty() {
        return false;
    }

    for key in keys {
        decrement_exact(disabled, key.as_str());
    }

    true
}

fn department_disable_matches(disabled: &HashMap<String, usize>, cop: &str) -> bool {
    if let Some((dept, _)) = cop.split_once('/') {
        return contains_case_insensitive(disabled, dept);
    }

    SHORT_NAME_TO_QUALIFIED
        .get(cop)
        .into_iter()
        .flatten()
        .filter_map(|qualified| qualified.split_once('/').map(|(dept, _)| dept))
        .any(|dept| contains_case_insensitive(disabled, dept))
}

fn decrement_case_insensitive(disabled: &mut HashMap<String, usize>, key: &str) -> bool {
    let actual = disabled
        .iter()
        .find(|(name, count)| **count > 0 && name.eq_ignore_ascii_case(key))
        .map(|(name, _)| name.clone());
    let Some(actual) = actual else {
        return false;
    };
    decrement_exact(disabled, actual.as_str());
    true
}

fn decrement_exact(disabled: &mut HashMap<String, usize>, key: &str) {
    let mut remove_key = false;
    if let Some(count) = disabled.get_mut(key) {
        *count -= 1;
        remove_key = *count == 0;
    }
    if remove_key {
        disabled.remove(key);
    }
}

fn contains_case_insensitive(disabled: &HashMap<String, usize>, key: &str) -> bool {
    disabled
        .iter()
        .any(|(name, count)| *count > 0 && name.eq_ignore_ascii_case(key))
}

fn short_cop_name(cop: &str) -> Option<&str> {
    cop.split_once('/').map(|(_, short)| short)
}

fn starts_with_case_insensitive(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

fn first_comment_hash(line: &str, byte_offset: usize, code_map: &CodeMap) -> Option<usize> {
    line.char_indices()
        .find(|(idx, ch)| *ch == '#' && code_map.is_not_string(byte_offset + idx))
        .map(|(idx, _)| idx)
}

fn find_directive_start(line: &str, first_comment_hash: Option<usize>) -> Option<usize> {
    let mut search_from = 0;
    let first_hash = first_comment_hash?;

    loop {
        let rest = &line[search_from..];
        let hash_pos = rest.find('#')?;
        let abs_pos = search_from + hash_pos;
        let after_hash = rest[hash_pos + 1..].trim_start();

        if strip_rubocop_prefix(after_hash).is_some() {
            let before = &line[..abs_pos];
            let before_trimmed = before.trim_end();
            if before_trimmed.ends_with('"')
                || before_trimmed.ends_with('\'')
                || before_trimmed.ends_with('`')
            {
                search_from = abs_pos + 1;
                continue;
            }

            if abs_pos != first_hash {
                let between_hashes = &line[first_hash + 1..abs_pos];
                if between_hashes.trim().is_empty() {
                    search_from = abs_pos + 1;
                    continue;
                }
            }

            return Some(abs_pos);
        }

        search_from = abs_pos + 1;
    }
}

fn strip_rubocop_prefix(s: &str) -> Option<&str> {
    let rest = s.strip_prefix("rubocop")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(':')?;
    Some(rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantCopEnableDirective,
        "cops/lint/redundant_cop_enable_directive"
    );

    #[test]
    fn finds_only_real_directive_start() {
        assert_eq!(
            find_directive_start("# rubocop: enable Metrics/MethodLength", Some(0)),
            Some(0)
        );
        assert_eq!(
            find_directive_start("value # rubocop:disable MethodLength", Some(6)),
            Some(6)
        );
        assert_eq!(
            find_directive_start("#   # rubocop:enable Layout/LineLength", Some(0)),
            None
        );
    }
}
