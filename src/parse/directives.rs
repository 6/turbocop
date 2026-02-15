use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::parse::source::SourceFile;

static DIRECTIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#\s*(?:rubocop|rblint)\s*:\s*(disable|enable|todo)\s+(.+)").unwrap()
});

/// Tracks line ranges where cops are disabled via inline comments.
///
/// Supports `# rubocop:disable`, `# rubocop:enable`, `# rubocop:todo`,
/// and the `# rblint:` equivalents.
pub struct DisabledRanges {
    /// Map from cop name (e.g. "Layout/LineLength"), department (e.g. "Metrics"),
    /// or "all" to disabled line ranges. Each range is (start_line, end_line)
    /// inclusive, 1-indexed.
    ranges: HashMap<String, Vec<(usize, usize)>>,
    /// If true, no directives were found — skip filtering entirely.
    empty: bool,
}

impl DisabledRanges {
    pub fn from_comments(source: &SourceFile, parse_result: &ruby_prism::ParseResult<'_>) -> Self {
        let mut ranges: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        let mut open_disables: HashMap<String, usize> = HashMap::new();
        let mut found_any = false;

        let lines: Vec<&[u8]> = source.lines().collect();

        for comment in parse_result.comments() {
            let loc = comment.location();
            let comment_bytes = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let Ok(comment_str) = std::str::from_utf8(comment_bytes) else {
                continue;
            };

            let Some(caps) = DIRECTIVE_RE.captures(comment_str) else {
                continue;
            };

            found_any = true;

            let action = &caps[1];
            let cop_list_raw = &caps[2];

            // Strip trailing comment marker (-- reason)
            let cop_list = match cop_list_raw.find("--") {
                Some(idx) => &cop_list_raw[..idx],
                None => cop_list_raw,
            };

            let (line, col) = source.offset_to_line_col(loc.start_offset());

            // Determine if inline: check if there's non-whitespace before the comment
            let is_inline = if line >= 1 && line <= lines.len() {
                let line_bytes = lines[line - 1];
                let before_comment = &line_bytes[..col.min(line_bytes.len())];
                before_comment.iter().any(|b| !b.is_ascii_whitespace())
            } else {
                false
            };

            let cop_names: Vec<&str> = cop_list
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            match action {
                "disable" | "todo" => {
                    for &cop in &cop_names {
                        if is_inline {
                            ranges.entry(cop.to_string()).or_default().push((line, line));
                        } else {
                            open_disables.insert(cop.to_string(), line);
                        }
                    }
                }
                "enable" => {
                    for &cop in &cop_names {
                        if let Some(start_line) = open_disables.remove(cop) {
                            ranges
                                .entry(cop.to_string())
                                .or_default()
                                .push((start_line, line));
                        }
                        // Orphaned enable without prior disable: ignore
                    }
                }
                _ => {}
            }
        }

        // Close any remaining open disables to EOF
        for (cop, start_line) in open_disables {
            ranges.entry(cop).or_default().push((start_line, usize::MAX));
        }

        DisabledRanges {
            ranges,
            empty: !found_any,
        }
    }

    /// Returns true if `cop_name` is disabled at `line`.
    ///
    /// Checks the exact cop name, its department prefix, and "all".
    pub fn is_disabled(&self, cop_name: &str, line: usize) -> bool {
        // Check exact cop name
        if self.check_ranges(cop_name, line) {
            return true;
        }

        // Check department name (e.g., "Layout" for "Layout/LineLength")
        if let Some(dept) = cop_name.split('/').next() {
            if dept != cop_name && self.check_ranges(dept, line) {
                return true;
            }
        }

        // Check "all"
        self.check_ranges("all", line)
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }

    fn check_ranges(&self, key: &str, line: usize) -> bool {
        if let Some(ranges) = self.ranges.get(key) {
            for &(start, end) in ranges {
                if line >= start && line <= end {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::source::SourceFile;

    fn disabled_ranges(src: &str) -> DisabledRanges {
        let source = SourceFile::from_bytes("test.rb", src.as_bytes().to_vec());
        let parse_result = crate::parse::parse_source(source.as_bytes());
        DisabledRanges::from_comments(&source, &parse_result)
    }

    #[test]
    fn single_line_inline_disable() {
        let dr = disabled_ranges("x = 1 # rubocop:disable Foo/Bar\ny = 2\n");
        assert!(!dr.is_empty());
        assert!(dr.is_disabled("Foo/Bar", 1));
        assert!(!dr.is_disabled("Foo/Bar", 2));
    }

    #[test]
    fn block_disable_enable() {
        let src = "# rubocop:disable Foo/Bar\nx = 1\ny = 2\n# rubocop:enable Foo/Bar\nz = 3\n";
        let dr = disabled_ranges(src);
        assert!(dr.is_disabled("Foo/Bar", 1));
        assert!(dr.is_disabled("Foo/Bar", 2));
        assert!(dr.is_disabled("Foo/Bar", 3));
        assert!(dr.is_disabled("Foo/Bar", 4));
        assert!(!dr.is_disabled("Foo/Bar", 5));
    }

    #[test]
    fn unterminated_disable() {
        let src = "# rubocop:disable Foo/Bar\nx = 1\ny = 2\n";
        let dr = disabled_ranges(src);
        assert!(dr.is_disabled("Foo/Bar", 1));
        assert!(dr.is_disabled("Foo/Bar", 2));
        assert!(dr.is_disabled("Foo/Bar", 3));
        assert!(dr.is_disabled("Foo/Bar", 999_999));
    }

    #[test]
    fn multiple_cops() {
        let src = "x = 1 # rubocop:disable Foo/Bar, Baz/Qux\ny = 2\n";
        let dr = disabled_ranges(src);
        assert!(dr.is_disabled("Foo/Bar", 1));
        assert!(dr.is_disabled("Baz/Qux", 1));
        assert!(!dr.is_disabled("Foo/Bar", 2));
        assert!(!dr.is_disabled("Baz/Qux", 2));
    }

    #[test]
    fn department_disable() {
        let src =
            "# rubocop:disable Metrics\nx = 1\n# rubocop:enable Metrics\ny = 2\n";
        let dr = disabled_ranges(src);
        assert!(dr.is_disabled("Metrics/MethodLength", 2));
        assert!(dr.is_disabled("Metrics/AbcSize", 2));
        assert!(!dr.is_disabled("Layout/LineLength", 2));
        // Enable line itself is still in the disabled range
        assert!(dr.is_disabled("Metrics/MethodLength", 3));
        // Line after enable is no longer disabled
        assert!(!dr.is_disabled("Metrics/MethodLength", 4));
    }

    #[test]
    fn disable_all() {
        let src = "# rubocop:disable all\nx = 1\n# rubocop:enable all\ny = 2\n";
        let dr = disabled_ranges(src);
        assert!(dr.is_disabled("Layout/LineLength", 2));
        assert!(dr.is_disabled("Style/Foo", 2));
        assert!(!dr.is_disabled("Layout/LineLength", 4));
    }

    #[test]
    fn rblint_alias() {
        let dr = disabled_ranges("x = 1 # rblint:disable Foo/Bar\ny = 2\n");
        assert!(dr.is_disabled("Foo/Bar", 1));
        assert!(!dr.is_disabled("Foo/Bar", 2));
    }

    #[test]
    fn todo_acts_as_disable() {
        let src = "# rubocop:todo Foo/Bar\nx = 1\n# rubocop:enable Foo/Bar\ny = 2\n";
        let dr = disabled_ranges(src);
        assert!(dr.is_disabled("Foo/Bar", 1));
        assert!(dr.is_disabled("Foo/Bar", 2));
        assert!(dr.is_disabled("Foo/Bar", 3));
        assert!(!dr.is_disabled("Foo/Bar", 4));
    }

    #[test]
    fn trailing_comment_marker() {
        let src = "x = 1 # rubocop:disable Foo/Bar -- reason why\ny = 2\n";
        let dr = disabled_ranges(src);
        assert!(dr.is_disabled("Foo/Bar", 1));
        assert!(!dr.is_disabled("Foo/Bar", 2));
        // "-- reason why" should not be parsed as a cop name
        assert!(!dr.is_disabled("-- reason why", 1));
    }

    #[test]
    fn no_space_after_hash() {
        let dr = disabled_ranges("x = 1 #rubocop:disable Foo/Bar\ny = 2\n");
        assert!(dr.is_disabled("Foo/Bar", 1));
    }

    #[test]
    fn extra_whitespace() {
        let dr = disabled_ranges("x = 1 # rubocop : disable Foo/Bar\ny = 2\n");
        assert!(dr.is_disabled("Foo/Bar", 1));
    }

    #[test]
    fn no_directives() {
        let dr = disabled_ranges("x = 1\ny = 2\n");
        assert!(dr.is_empty());
        assert!(!dr.is_disabled("Foo/Bar", 1));
    }

    #[test]
    fn orphaned_enable_ignored() {
        let dr = disabled_ranges("# rubocop:enable Foo/Bar\nx = 1\n");
        assert!(!dr.is_disabled("Foo/Bar", 1));
        assert!(!dr.is_disabled("Foo/Bar", 2));
    }

    #[test]
    fn inline_disable_only_affects_that_line() {
        let src = "a = 1 # rubocop:disable Layout/LineLength\nb = 2\nc = 3\n";
        let dr = disabled_ranges(src);
        assert!(dr.is_disabled("Layout/LineLength", 1));
        assert!(!dr.is_disabled("Layout/LineLength", 2));
        assert!(!dr.is_disabled("Layout/LineLength", 3));
    }

    #[test]
    fn standalone_disable_is_range() {
        // A disable on its own line (no code before it) starts a range
        let src = "  # rubocop:disable Layout/LineLength\nb = 2\nc = 3\n  # rubocop:enable Layout/LineLength\nd = 4\n";
        let dr = disabled_ranges(src);
        assert!(dr.is_disabled("Layout/LineLength", 1));
        assert!(dr.is_disabled("Layout/LineLength", 2));
        assert!(dr.is_disabled("Layout/LineLength", 3));
        assert!(dr.is_disabled("Layout/LineLength", 4));
        assert!(!dr.is_disabled("Layout/LineLength", 5));
    }
}
