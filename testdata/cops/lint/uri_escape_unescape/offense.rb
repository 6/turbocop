URI.escape("http://example.com")
    ^^^^^^ Lint/UriEscapeUnescape: `URI.escape` method is obsolete and should not be used.
URI.unescape("%20")
    ^^^^^^^^ Lint/UriEscapeUnescape: `URI.unescape` method is obsolete and should not be used.
URI.escape("another")
    ^^^^^^ Lint/UriEscapeUnescape: `URI.escape` method is obsolete and should not be used.
::URI.escape("qualified")
      ^^^^^^ Lint/UriEscapeUnescape: `URI.escape` method is obsolete and should not be used.
::URI.unescape("qualified")
      ^^^^^^^^ Lint/UriEscapeUnescape: `URI.unescape` method is obsolete and should not be used.
