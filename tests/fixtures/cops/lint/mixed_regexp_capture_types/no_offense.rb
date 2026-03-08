/(?<foo>bar)(?<baz>qux)/
/(foo)(bar)/
/(?<foo>bar)(?:baz)/
/(?<=>)(<br>)(?=><)/
/no captures here/
# Named capture with parens inside a character class (not a capture group)
/\$(?<cmd>\((?:[^()]|\g<cmd>)+\))/
# Conditional backreference with angle brackets (not a capture group)
/(?<a>a)(?(<a>)a|b)/
# Conditional backreference with single quotes (not a capture group)
/(?<a>a)(?('a')a|b)/
# Extended mode: comments with parens should not count as capture groups
/
  (?<version>\d+\.\d+\.\d+)           # major.minor.patch (e.g., 1.2.3)
  (?:-(?<prerelease>[a-zA-Z0-9.-]+))? # optional prerelease (e.g., -alpha.1)
/x
