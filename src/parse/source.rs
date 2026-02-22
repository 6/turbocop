use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::diagnostic::Location;

#[derive(Debug)]
pub struct SourceFile {
    pub path: PathBuf,
    pub content: Vec<u8>,
    /// Byte offsets where each line starts (0-indexed into content)
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let content =
            std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
        let line_starts = compute_line_starts(&content);
        Ok(Self {
            path: path.to_path_buf(),
            content,
            line_starts,
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.content
    }

    /// Returns an iterator over lines as byte slices (without newline terminators).
    pub fn lines(&self) -> impl Iterator<Item = &[u8]> {
        self.content.split(|&b| b == b'\n')
    }

    /// Convert a byte offset into a (1-indexed line, 0-indexed column) pair.
    /// Column is a character offset (UTF-8 codepoint count) within the line.
    pub fn offset_to_line_col(&self, byte_offset: usize) -> (usize, usize) {
        let line_idx = match self.line_starts.binary_search(&byte_offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        let line_bytes = &self.content[self.line_starts[line_idx]..byte_offset];
        // Count bytes that are NOT UTF-8 continuation bytes (0x80..0xBF).
        // This equals the number of UTF-8 character starts, and works correctly
        // even for partial or invalid UTF-8.
        let col = line_bytes.iter().filter(|&&b| (b & 0xC0) != 0x80).count();
        (line_idx + 1, col)
    }

    /// Convert a ruby_prism::Location into our diagnostic::Location.
    pub fn prism_location_to_location(&self, loc: &ruby_prism::Location<'_>) -> Location {
        let (line, column) = self.offset_to_line_col(loc.start_offset());
        Location { line, column }
    }

    /// Convert a (1-indexed line, 0-indexed column) pair to a byte offset.
    /// Column is a character offset (UTF-8 codepoint count) within the line.
    /// Returns `None` if line is out of range.
    pub fn line_col_to_offset(&self, line: usize, col: usize) -> Option<usize> {
        if line == 0 || line > self.line_starts.len() {
            return None;
        }
        let start = self.line_starts[line - 1];
        if col == 0 {
            return Some(start);
        }
        let end = if line < self.line_starts.len() {
            self.line_starts[line]
        } else {
            self.content.len()
        };
        let mut chars_seen = 0;
        for (i, &b) in self.content[start..end].iter().enumerate() {
            // Only check at character boundaries (non-continuation bytes)
            if (b & 0xC0) != 0x80 {
                if chars_seen == col {
                    return Some(start + i);
                }
                chars_seen += 1;
            }
        }
        if chars_seen == col {
            Some(end)
        } else {
            // col exceeds line length; fall back to clamped position
            Some(start + col.min(end - start))
        }
    }

    pub fn path_str(&self) -> &str {
        self.path.to_str().unwrap_or("<non-utf8 path>")
    }

    /// Create a SourceFile from a string, using the given path for display purposes.
    pub fn from_string(path: PathBuf, content: String) -> Self {
        let bytes = content.into_bytes();
        let line_starts = compute_line_starts(&bytes);
        Self {
            path,
            content: bytes,
            line_starts,
        }
    }

    /// Create a SourceFile from raw bytes and a path.
    pub fn from_vec(path: PathBuf, content: Vec<u8>) -> Self {
        let line_starts = compute_line_starts(&content);
        Self {
            path,
            content,
            line_starts,
        }
    }

    /// Create a SourceFile from raw bytes (for testing).
    #[cfg(test)]
    pub fn from_bytes(path: &str, content: Vec<u8>) -> Self {
        let line_starts = compute_line_starts(&content);
        Self {
            path: PathBuf::from(path),
            content,
            line_starts,
        }
    }
}

fn compute_line_starts(content: &[u8]) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, &byte) in content.iter().enumerate() {
        if byte == b'\n' && i + 1 < content.len() {
            starts.push(i + 1);
        }
    }
    starts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(s: &str) -> SourceFile {
        SourceFile::from_bytes("test.rb", s.as_bytes().to_vec())
    }

    #[test]
    fn line_starts_single_line() {
        let sf = source("hello");
        assert_eq!(sf.line_starts, vec![0]);
    }

    #[test]
    fn line_starts_multiple_lines() {
        // "abc\ndef\nghi"
        // 0123 4567 89..
        let sf = source("abc\ndef\nghi");
        assert_eq!(sf.line_starts, vec![0, 4, 8]);
    }

    #[test]
    fn line_starts_trailing_newline() {
        // "abc\n" — no line start after the last \n since there's no content
        let sf = source("abc\n");
        assert_eq!(sf.line_starts, vec![0]);
    }

    #[test]
    fn offset_to_line_col_first_char() {
        let sf = source("abc\ndef\nghi");
        assert_eq!(sf.offset_to_line_col(0), (1, 0));
    }

    #[test]
    fn offset_to_line_col_mid_first_line() {
        let sf = source("abc\ndef\nghi");
        assert_eq!(sf.offset_to_line_col(2), (1, 2));
    }

    #[test]
    fn offset_to_line_col_second_line_start() {
        let sf = source("abc\ndef\nghi");
        // byte 4 = 'd', line 2, col 0
        assert_eq!(sf.offset_to_line_col(4), (2, 0));
    }

    #[test]
    fn offset_to_line_col_third_line() {
        let sf = source("abc\ndef\nghi");
        // byte 9 = 'h' (wait: 8='g', 9='h', 10='i')
        assert_eq!(sf.offset_to_line_col(9), (3, 1));
    }

    #[test]
    fn lines_iterator() {
        let sf = source("abc\ndef\nghi");
        let lines: Vec<&[u8]> = sf.lines().collect();
        assert_eq!(lines, vec![b"abc", b"def", b"ghi"]);
    }

    #[test]
    fn lines_trailing_newline() {
        let sf = source("abc\n");
        let lines: Vec<&[u8]> = sf.lines().collect();
        assert_eq!(lines, vec![b"abc".as_slice(), b"".as_slice()]);
    }

    #[test]
    fn as_bytes_roundtrip() {
        let sf = source("puts 'hi'");
        assert_eq!(sf.as_bytes(), b"puts 'hi'");
    }

    #[test]
    fn from_path_reads_file() {
        let dir = std::env::temp_dir().join("turbocop_test_source");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("test.rb");
        std::fs::write(&file, b"x = 1\n").unwrap();
        let sf = SourceFile::from_path(&file).unwrap();
        assert_eq!(sf.as_bytes(), b"x = 1\n");
        assert_eq!(sf.path, file);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn line_col_to_offset_basic() {
        let sf = source("abc\ndef\nghi");
        assert_eq!(sf.line_col_to_offset(1, 0), Some(0));
        assert_eq!(sf.line_col_to_offset(1, 2), Some(2));
        assert_eq!(sf.line_col_to_offset(2, 0), Some(4));
        assert_eq!(sf.line_col_to_offset(3, 1), Some(9));
    }

    #[test]
    fn line_col_to_offset_out_of_range() {
        let sf = source("abc\ndef");
        assert_eq!(sf.line_col_to_offset(0, 0), None);
        assert_eq!(sf.line_col_to_offset(3, 0), None); // only 2 lines
    }

    #[test]
    fn from_path_nonexistent() {
        let result = SourceFile::from_path(Path::new("/nonexistent/file.rb"));
        assert!(result.is_err());
    }

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn line_starts_first_is_zero(content in prop::collection::vec(any::<u8>(), 0..500)) {
                let starts = compute_line_starts(&content);
                prop_assert_eq!(starts[0], 0, "first line start must be 0");
            }

            #[test]
            fn line_starts_are_strictly_increasing(content in prop::collection::vec(any::<u8>(), 0..500)) {
                let starts = compute_line_starts(&content);
                for pair in starts.windows(2) {
                    prop_assert!(pair[0] < pair[1],
                        "line starts not strictly increasing: {} >= {}", pair[0], pair[1]);
                }
            }

            #[test]
            fn line_starts_follow_newlines(content in prop::collection::vec(any::<u8>(), 0..500)) {
                let starts = compute_line_starts(&content);
                // Every start after the first should be immediately after a \n
                for &start in &starts[1..] {
                    prop_assert!(start > 0 && content[start - 1] == b'\n',
                        "line start {} is not preceded by newline", start);
                }
            }

            #[test]
            fn offset_to_line_col_roundtrip(content in "[\\x00-\\x7f\\u{80}-\\u{10FFFF}]{1,200}") {
                let bytes = content.as_bytes().to_vec();
                let sf = SourceFile::from_bytes("test.rb", bytes.clone());
                // Test every valid byte offset that falls on a UTF-8 character boundary
                for offset in 0..bytes.len() {
                    if !content.is_char_boundary(offset) {
                        continue; // skip offsets in the middle of multi-byte chars
                    }
                    let (line, col) = sf.offset_to_line_col(offset);
                    let reconstructed = sf.line_col_to_offset(line, col);
                    prop_assert_eq!(Some(offset), reconstructed,
                        "round-trip failed: offset {} -> ({}, {}) -> {:?}",
                        offset, line, col, reconstructed);
                }
            }

            #[test]
            fn offset_to_line_col_line_in_range(content in prop::collection::vec(any::<u8>(), 1..500)) {
                let sf = SourceFile::from_bytes("test.rb", content.clone());
                let num_lines = sf.line_starts.len();
                for offset in 0..content.len() {
                    let (line, _col) = sf.offset_to_line_col(offset);
                    prop_assert!(line >= 1 && line <= num_lines,
                        "line {} out of range [1, {}] for offset {}",
                        line, num_lines, offset);
                }
            }

            #[test]
            fn line_col_to_offset_roundtrip(content in "[\\x00-\\x7f\\u{80}-\\u{10FFFF}]{1,200}") {
                let bytes = content.as_bytes().to_vec();
                let sf = SourceFile::from_bytes("test.rb", bytes.clone());
                for offset in 0..bytes.len() {
                    if !content.is_char_boundary(offset) {
                        continue; // skip offsets in the middle of multi-byte chars
                    }
                    let (line, col) = sf.offset_to_line_col(offset);
                    let back = sf.line_col_to_offset(line, col);
                    prop_assert_eq!(back, Some(offset),
                        "roundtrip failed: offset {} -> ({}, {}) -> {:?}",
                        offset, line, col, back);
                }
            }

            #[test]
            fn offset_to_line_col_is_monotonic(content in prop::collection::vec(any::<u8>(), 1..500)) {
                let sf = SourceFile::from_bytes("test.rb", content.clone());
                let mut prev = (0usize, 0usize);
                for offset in 0..content.len() {
                    let cur = sf.offset_to_line_col(offset);
                    prop_assert!(cur >= prev,
                        "monotonicity violated: offset {} -> {:?} but offset {} -> {:?}",
                        offset, cur, offset.saturating_sub(1), prev);
                    prev = cur;
                }
            }
        }
    }
}
