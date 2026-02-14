use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FrozenStringLiteralComment;

impl Cop for FrozenStringLiteralComment {
    fn name(&self) -> &'static str {
        "Style/FrozenStringLiteralComment"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let lines: Vec<&[u8]> = source.lines().collect();

        if lines.is_empty() || (lines.len() == 1 && lines[0].is_empty()) {
            return vec![self.diagnostic(source, 1, 0, "Missing frozen string literal comment.".to_string())];
        }

        let mut idx = 0;

        // Skip shebang
        if idx < lines.len() && lines[idx].starts_with(b"#!") {
            idx += 1;
        }

        // Skip encoding comment
        if idx < lines.len() && is_encoding_comment(lines[idx]) {
            idx += 1;
        }

        // Check for frozen_string_literal magic comment
        if idx < lines.len() && is_frozen_string_literal_comment(lines[idx]) {
            return Vec::new();
        }

        vec![self.diagnostic(source, 1, 0, "Missing frozen string literal comment.".to_string())]
    }
}

fn is_encoding_comment(line: &[u8]) -> bool {
    let s = match std::str::from_utf8(line) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.starts_with("# encoding:")
        || s.starts_with("# coding:")
        || (s.starts_with("# -*-") && (s.contains("encoding:") || s.contains("coding:")))
}

fn is_frozen_string_literal_comment(line: &[u8]) -> bool {
    let s = match std::str::from_utf8(line) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.starts_with("# frozen_string_literal:")
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(FrozenStringLiteralComment, "cops/style/frozen_string_literal_comment");

    #[test]
    fn missing_comment() {
        let source = SourceFile::from_bytes("test.rb", b"puts 'hello'\n".to_vec());
        let diags = FrozenStringLiteralComment.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 0);
        assert_eq!(diags[0].message, "Missing frozen string literal comment.");
    }

    #[test]
    fn with_frozen_true() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let diags = FrozenStringLiteralComment.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn with_frozen_false() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# frozen_string_literal: false\nputs 'hello'\n".to_vec(),
        );
        let diags = FrozenStringLiteralComment.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn with_shebang_and_frozen() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"#!/usr/bin/env ruby\n# frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let diags = FrozenStringLiteralComment.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn with_shebang_no_frozen() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"#!/usr/bin/env ruby\nputs 'hello'\n".to_vec(),
        );
        let diags = FrozenStringLiteralComment.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn with_encoding_and_frozen() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# encoding: utf-8\n# frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let diags = FrozenStringLiteralComment.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn with_shebang_encoding_and_frozen() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"#!/usr/bin/env ruby\n# encoding: utf-8\n# frozen_string_literal: true\nputs 'hello'\n"
                .to_vec(),
        );
        let diags = FrozenStringLiteralComment.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn empty_file() {
        let source = SourceFile::from_bytes("test.rb", b"".to_vec());
        let diags = FrozenStringLiteralComment.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn emacs_encoding_style() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# -*- coding: utf-8 -*-\n# frozen_string_literal: true\nputs 'hello'\n".to_vec(),
        );
        let diags = FrozenStringLiteralComment.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }
}
