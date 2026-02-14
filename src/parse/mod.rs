pub mod source;

/// Parse Ruby source bytes using Prism.
///
/// This must be called on the thread that will use the result, since
/// `ParseResult` is `!Send + !Sync`.
pub fn parse_source(source: &[u8]) -> ruby_prism::ParseResult<'_> {
    ruby_prism::parse(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_ruby() {
        let result = parse_source(b"puts 'hello'");
        assert_eq!(result.errors().count(), 0);
    }

    #[test]
    fn parse_empty_source() {
        let result = parse_source(b"");
        assert_eq!(result.errors().count(), 0);
    }

    #[test]
    fn parse_syntax_error_still_returns() {
        let result = parse_source(b"def foo(");
        assert!(result.errors().count() > 0);
    }
}
