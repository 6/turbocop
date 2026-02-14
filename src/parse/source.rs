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
    pub fn offset_to_line_col(&self, byte_offset: usize) -> (usize, usize) {
        let line_idx = match self.line_starts.binary_search(&byte_offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        let col = byte_offset - self.line_starts[line_idx];
        (line_idx + 1, col)
    }

    /// Convert a ruby_prism::Location into our diagnostic::Location.
    pub fn prism_location_to_location(&self, loc: &ruby_prism::Location<'_>) -> Location {
        let (line, column) = self.offset_to_line_col(loc.start_offset());
        Location { line, column }
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
        let dir = std::env::temp_dir().join("rblint_test_source");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("test.rb");
        std::fs::write(&file, b"x = 1\n").unwrap();
        let sf = SourceFile::from_path(&file).unwrap();
        assert_eq!(sf.as_bytes(), b"x = 1\n");
        assert_eq!(sf.path, file);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn from_path_nonexistent() {
        let result = SourceFile::from_path(Path::new("/nonexistent/file.rb"));
        assert!(result.is_err());
    }
}
