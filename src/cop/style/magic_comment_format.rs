use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MagicCommentFormat;

const MAGIC_COMMENT_DIRECTIVES: &[&str] = &[
    "frozen_string_literal",
    "frozen-string-literal",
    "encoding",
    "shareable_constant_value",
    "shareable-constant-value",
    "typed",
    "warn_indent",
    "warn-indent",
];

impl MagicCommentFormat {
    fn is_magic_comment_directive(word: &str) -> bool {
        let normalized = word.replace('-', "_").replace('_', "_").to_lowercase();
        MAGIC_COMMENT_DIRECTIVES
            .iter()
            .any(|&d| d.replace('-', "_").to_lowercase() == normalized)
    }

    fn has_underscores(s: &str) -> bool {
        s.contains('_')
    }

    fn has_dashes(s: &str) -> bool {
        s.contains('-')
    }
}

impl Cop for MagicCommentFormat {
    fn name(&self) -> &'static str {
        "Style/MagicCommentFormat"
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let lines: Vec<&str> = source.lines()
            .filter_map(|l| std::str::from_utf8(l).ok())
            .collect();
        let style = config.get_str("EnforcedStyle", "snake_case");
        let _directive_cap = config.get_str("DirectiveCapitalization", "");
        let _value_cap = config.get_str("ValueCapitalization", "");

        // Only check lines before the first code statement
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Stop at first non-comment, non-blank line
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                break;
            }

            if !trimmed.starts_with('#') {
                continue;
            }

            let content = &trimmed[1..].trim_start();

            // Handle emacs-style: # -*- key: value; key: value -*-
            let is_emacs = content.starts_with("-*-");

            if is_emacs {
                // Parse emacs-style directives
                let inner = content.trim_start_matches("-*-").trim_end_matches("-*-").trim();
                for part in inner.split(';') {
                    let part = part.trim();
                    if let Some(colon_pos) = part.find(':') {
                        let directive = part[..colon_pos].trim();
                        if Self::is_magic_comment_directive(directive) {
                            Self::check_directive_style(diagnostics, source, i, line, directive, style, self);
                        }
                    }
                }
            } else {
                // Standard style: # directive: value
                if let Some(colon_pos) = content.find(':') {
                    let directive = content[..colon_pos].trim();
                    if Self::is_magic_comment_directive(directive) {
                        Self::check_directive_style(diagnostics, source, i, line, directive, style, self);
                    }
                }
            }
        }

    }
}

impl MagicCommentFormat {
    fn check_directive_style(
        diagnostics: &mut Vec<Diagnostic>,
        source: &SourceFile,
        line_idx: usize,
        line: &str,
        directive: &str,
        style: &str,
        cop: &MagicCommentFormat,
    ) {
        // Directives that can vary: frozen_string_literal / frozen-string-literal
        // encoding doesn't vary
        // shareable_constant_value / shareable-constant-value
        // typed doesn't vary
        if !Self::has_underscores(directive) && !Self::has_dashes(directive) {
            return;
        }

        let wrong = match style {
            "snake_case" => Self::has_dashes(directive),
            "kebab_case" => Self::has_underscores(directive),
            _ => false,
        };

        if wrong {
            // Find the directive position in the line
            if let Some(pos) = line.find(directive) {
                let line_num = line_idx + 1;
                let msg = match style {
                    "snake_case" => "Prefer snake case for magic comments.".to_string(),
                    "kebab_case" => "Prefer kebab case for magic comments.".to_string(),
                    _ => return,
                };
                diagnostics.push(cop.diagnostic(
                    source,
                    line_num,
                    pos,
                    msg,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MagicCommentFormat, "cops/style/magic_comment_format");
}
