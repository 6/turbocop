use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::parse::source::SourceFile;

static DIRECTIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#\s*(?:rubocop|rblint)\s*:\s*(disable|enable|todo)\s+(.+)").unwrap()
});

/// A single disable directive entry (one cop name from a `# rubocop:disable` comment).
#[derive(Debug, Clone)]
pub struct DisableDirective {
    /// The cop/department/all name exactly as written in the comment.
    pub cop_name: String,
    /// 1-indexed line number of the directive comment.
    pub line: usize,
    /// 0-indexed column of the `#` starting the comment.
    pub column: usize,
    /// Whether this directive is inline (code before the `#` on the same line).
    pub is_inline: bool,
    /// The line range this directive covers (start_line, end_line) inclusive, 1-indexed.
    pub range: (usize, usize),
    /// Whether this directive actually suppressed at least one diagnostic.
    pub used: bool,
}

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
    /// All disable directives found, for redundancy checking.
    directives: Vec<DisableDirective>,
}

impl DisabledRanges {
    pub fn from_comments(source: &SourceFile, parse_result: &ruby_prism::ParseResult<'_>) -> Self {
        let mut ranges: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        // Track open block disables: cop_name -> (start_line, column, directive_index)
        let mut open_disables: HashMap<String, (usize, usize, usize)> = HashMap::new();
        let mut found_any = false;
        let mut directives: Vec<DisableDirective> = Vec::new();

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
                .map(|s| {
                    // Extract just the cop name, ignoring trailing free-text comments.
                    // Cop names are: "all", "Department", or "Department/CopName".
                    // If there's a space after the cop name, the rest is a comment.
                    match s.find(' ') {
                        Some(idx) => &s[..idx],
                        None => s,
                    }
                })
                .filter(|s| !s.is_empty())
                .collect();

            match action {
                "disable" | "todo" => {
                    for &cop in &cop_names {
                        if is_inline {
                            let range = (line, line);
                            ranges.entry(cop.to_string()).or_default().push(range);
                            directives.push(DisableDirective {
                                cop_name: cop.to_string(),
                                line,
                                column: col,
                                is_inline: true,
                                range,
                                used: false,
                            });
                        } else {
                            let directive_idx = directives.len();
                            directives.push(DisableDirective {
                                cop_name: cop.to_string(),
                                line,
                                column: col,
                                is_inline: false,
                                range: (line, usize::MAX), // placeholder, updated on enable/EOF
                                used: false,
                            });
                            open_disables.insert(cop.to_string(), (line, col, directive_idx));
                        }
                    }
                }
                "enable" => {
                    for &cop in &cop_names {
                        if let Some((start_line, _col, directive_idx)) = open_disables.remove(cop) {
                            let range = (start_line, line);
                            ranges
                                .entry(cop.to_string())
                                .or_default()
                                .push(range);
                            // Update the directive's range
                            if directive_idx < directives.len() {
                                directives[directive_idx].range = range;
                            }
                        }
                        // Orphaned enable without prior disable: ignore
                    }
                }
                _ => {}
            }
        }

        // Close any remaining open disables to EOF
        for (cop, (start_line, _col, directive_idx)) in open_disables {
            let range = (start_line, usize::MAX);
            ranges.entry(cop).or_default().push(range);
            if directive_idx < directives.len() {
                directives[directive_idx].range = range;
            }
        }

        DisabledRanges {
            ranges,
            empty: !found_any,
            directives,
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

    /// Check if a diagnostic is disabled AND mark the matching directive(s) as used.
    ///
    /// Returns true if the diagnostic should be suppressed.
    pub fn check_and_mark_used(&mut self, cop_name: &str, line: usize) -> bool {
        let mut suppressed = false;

        // Check exact cop name
        if self.check_ranges(cop_name, line) {
            self.mark_directives_used(cop_name, line);
            suppressed = true;
        }

        // Check department name (e.g., "Layout" for "Layout/LineLength")
        if let Some(dept) = cop_name.split('/').next() {
            if dept != cop_name && self.check_ranges(dept, line) {
                self.mark_directives_used(dept, line);
                suppressed = true;
            }
        }

        // Check "all"
        if self.check_ranges("all", line) {
            self.mark_directives_used("all", line);
            suppressed = true;
        }

        suppressed
    }

    /// Mark all directives with the given key that cover the given line as used.
    fn mark_directives_used(&mut self, key: &str, line: usize) {
        for directive in &mut self.directives {
            if directive.cop_name == key
                && line >= directive.range.0
                && line <= directive.range.1
            {
                directive.used = true;
            }
        }
    }

    /// Return all unused disable directives (those that didn't suppress any diagnostic).
    pub fn unused_directives(&self) -> impl Iterator<Item = &DisableDirective> {
        self.directives.iter().filter(|d| !d.used)
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Returns true if there are any disable directives (used for redundancy checking).
    pub fn has_directives(&self) -> bool {
        !self.directives.is_empty()
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

    // --- check_and_mark_used tests ---

    #[test]
    fn check_and_mark_used_marks_directive() {
        let mut dr = disabled_ranges("x = 1 # rubocop:disable Foo/Bar\ny = 2\n");
        assert!(dr.check_and_mark_used("Foo/Bar", 1));
        assert!(!dr.check_and_mark_used("Foo/Bar", 2));
        let unused: Vec<_> = dr.unused_directives().collect();
        assert!(unused.is_empty(), "directive should be marked used");
    }

    #[test]
    fn unused_directive_reported() {
        let mut dr = disabled_ranges("x = 1 # rubocop:disable Foo/Bar\ny = 2\n");
        // Never call check_and_mark_used -> directive stays unused
        let unused: Vec<_> = dr.unused_directives().collect();
        assert_eq!(unused.len(), 1);
        assert_eq!(unused[0].cop_name, "Foo/Bar");
        assert_eq!(unused[0].line, 1);
    }

    #[test]
    fn department_disable_marked_used() {
        let mut dr = disabled_ranges(
            "# rubocop:disable Metrics\nx = 1\n# rubocop:enable Metrics\ny = 2\n",
        );
        assert!(dr.check_and_mark_used("Metrics/MethodLength", 2));
        let unused: Vec<_> = dr.unused_directives().collect();
        assert!(unused.is_empty(), "department directive should be marked used");
    }

    #[test]
    fn all_disable_marked_used() {
        let mut dr = disabled_ranges(
            "# rubocop:disable all\nx = 1\n# rubocop:enable all\ny = 2\n",
        );
        assert!(dr.check_and_mark_used("Style/Foo", 2));
        let unused: Vec<_> = dr.unused_directives().collect();
        assert!(unused.is_empty(), "all directive should be marked used");
    }

    #[test]
    fn block_directive_unused() {
        let mut dr = disabled_ranges(
            "# rubocop:disable Foo/Bar\nx = 1\ny = 2\n# rubocop:enable Foo/Bar\nz = 3\n",
        );
        // No diagnostics suppressed
        let unused: Vec<_> = dr.unused_directives().collect();
        assert_eq!(unused.len(), 1);
        assert_eq!(unused[0].cop_name, "Foo/Bar");
        assert_eq!(unused[0].line, 1);
        assert!(!unused[0].is_inline);
    }

    #[test]
    fn multiple_cops_one_used_one_not() {
        let mut dr = disabled_ranges("x = 1 # rubocop:disable Foo/Bar, Baz/Qux\ny = 2\n");
        assert!(dr.check_and_mark_used("Foo/Bar", 1));
        let unused: Vec<_> = dr.unused_directives().collect();
        assert_eq!(unused.len(), 1);
        assert_eq!(unused[0].cop_name, "Baz/Qux");
    }
}
