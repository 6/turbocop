/// A single source-level edit: replace byte range [start..end) with replacement.
#[derive(Debug, Clone)]
pub struct Correction {
    /// Byte offset, inclusive.
    pub start: usize,
    /// Byte offset, exclusive.
    pub end: usize,
    /// Replacement text (empty string = deletion).
    pub replacement: String,
    /// Cop that produced this correction.
    pub cop_name: &'static str,
    /// Registry index for deterministic conflict resolution (lower wins).
    pub cop_index: usize,
}

/// A set of non-overlapping corrections, sorted by start offset.
///
/// Built from an unsorted vec of corrections. Overlapping corrections are
/// resolved by dropping the later one (matching RuboCop's "first merged wins"
/// behavior). When two corrections start at the same offset, the one from the
/// earlier cop in registry order wins.
pub struct CorrectionSet {
    corrections: Vec<Correction>,
}

impl CorrectionSet {
    /// Build from an unsorted vec of corrections.
    ///
    /// Sorts by (start, cop_index), then drops any correction whose range
    /// overlaps with the previously accepted correction.
    pub fn from_vec(mut raw: Vec<Correction>) -> Self {
        // Primary sort: start offset ascending.
        // Tiebreaker: cop_index ascending (lower registry index wins).
        raw.sort_by(|a, b| a.start.cmp(&b.start).then(a.cop_index.cmp(&b.cop_index)));

        let mut accepted: Vec<Correction> = Vec::with_capacity(raw.len());
        for c in raw {
            if let Some(last) = accepted.last() {
                if c.start < last.end {
                    // Overlaps with previous — drop this correction.
                    continue;
                }
            }
            accepted.push(c);
        }

        Self {
            corrections: accepted,
        }
    }

    /// Apply corrections to source bytes, returning new source.
    ///
    /// Uses a single O(n) linear scan:
    /// ```text
    /// cursor = 0
    /// for each correction c (sorted by start):
    ///     copy source[cursor..c.start]
    ///     copy c.replacement
    ///     cursor = c.end
    /// copy source[cursor..]
    /// ```
    pub fn apply(&self, source: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(source.len());
        let mut cursor = 0;

        for c in &self.corrections {
            // Copy unchanged bytes before this correction.
            if c.start > cursor {
                result.extend_from_slice(&source[cursor..c.start]);
            }
            // Copy replacement.
            result.extend_from_slice(c.replacement.as_bytes());
            cursor = c.end;
        }

        // Copy remaining bytes after last correction.
        if cursor < source.len() {
            result.extend_from_slice(&source[cursor..]);
        }

        result
    }

    pub fn is_empty(&self) -> bool {
        self.corrections.is_empty()
    }

    pub fn len(&self) -> usize {
        self.corrections.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn correction(start: usize, end: usize, replacement: &str, cop_index: usize) -> Correction {
        Correction {
            start,
            end,
            replacement: replacement.to_string(),
            cop_name: "Test/Cop",
            cop_index,
        }
    }

    #[test]
    fn empty_corrections_returns_source_unchanged() {
        let source = b"hello world";
        let cs = CorrectionSet::from_vec(vec![]);
        assert_eq!(cs.apply(source), source.to_vec());
        assert!(cs.is_empty());
        assert_eq!(cs.len(), 0);
    }

    #[test]
    fn single_deletion() {
        // "hello world" → "helloworld" (delete the space)
        let source = b"hello world";
        let cs = CorrectionSet::from_vec(vec![correction(5, 6, "", 0)]);
        assert_eq!(cs.apply(source), b"helloworld");
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn single_insertion() {
        // Insert at offset 5: "hello world" → "hello, world"
        let source = b"hello world";
        let cs = CorrectionSet::from_vec(vec![correction(5, 5, ",", 0)]);
        assert_eq!(cs.apply(source), b"hello, world");
    }

    #[test]
    fn single_replacement() {
        // Replace "world" with "rust": "hello world" → "hello rust"
        let source = b"hello world";
        let cs = CorrectionSet::from_vec(vec![correction(6, 11, "rust", 0)]);
        assert_eq!(cs.apply(source), b"hello rust");
    }

    #[test]
    fn multiple_non_overlapping() {
        // "abc def ghi" → "ABC def GHI"
        let source = b"abc def ghi";
        let cs = CorrectionSet::from_vec(vec![
            correction(8, 11, "GHI", 0),
            correction(0, 3, "ABC", 0),
        ]);
        assert_eq!(cs.apply(source), b"ABC def GHI");
        assert_eq!(cs.len(), 2);
    }

    #[test]
    fn overlapping_drops_second() {
        // Two corrections overlap: first one (by start offset) wins.
        let source = b"abcdefgh";
        let cs = CorrectionSet::from_vec(vec![
            correction(2, 6, "XX", 0), // replace "cdef"
            correction(4, 8, "YY", 1), // replace "efgh" — overlaps, should be dropped
        ]);
        assert_eq!(cs.apply(source), b"abXXgh");
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn same_start_cop_index_wins() {
        // Two corrections at same offset: lower cop_index wins.
        let source = b"abc";
        let cs = CorrectionSet::from_vec(vec![
            correction(0, 3, "LOSE", 5), // higher index
            correction(0, 3, "WIN", 1),  // lower index — should win
        ]);
        assert_eq!(cs.apply(source), b"WIN");
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn correction_at_start() {
        let source = b"abc";
        let cs = CorrectionSet::from_vec(vec![correction(0, 1, "X", 0)]);
        assert_eq!(cs.apply(source), b"Xbc");
    }

    #[test]
    fn correction_at_end() {
        let source = b"abc";
        let cs = CorrectionSet::from_vec(vec![correction(2, 3, "X", 0)]);
        assert_eq!(cs.apply(source), b"abX");
    }

    #[test]
    fn insertion_at_start() {
        let source = b"abc";
        let cs = CorrectionSet::from_vec(vec![correction(0, 0, "X", 0)]);
        assert_eq!(cs.apply(source), b"Xabc");
    }

    #[test]
    fn insertion_at_end() {
        let source = b"abc";
        let cs = CorrectionSet::from_vec(vec![correction(3, 3, "X", 0)]);
        assert_eq!(cs.apply(source), b"abcX");
    }

    #[test]
    fn adjacent_non_overlapping() {
        // Adjacent corrections (end of first == start of second) should both apply.
        let source = b"abcdef";
        let cs = CorrectionSet::from_vec(vec![
            correction(0, 3, "X", 0), // replace "abc"
            correction(3, 6, "Y", 0), // replace "def"
        ]);
        assert_eq!(cs.apply(source), b"XY");
        assert_eq!(cs.len(), 2);
    }

    #[test]
    fn delete_entire_source() {
        let source = b"abc";
        let cs = CorrectionSet::from_vec(vec![correction(0, 3, "", 0)]);
        assert_eq!(cs.apply(source), b"");
    }

    #[test]
    fn empty_source() {
        let source = b"";
        let cs = CorrectionSet::from_vec(vec![correction(0, 0, "hello", 0)]);
        assert_eq!(cs.apply(source), b"hello");
    }
}
