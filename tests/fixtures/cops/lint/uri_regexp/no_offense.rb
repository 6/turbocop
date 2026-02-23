URI::DEFAULT_PARSER.make_regexp
URI.parse("http://example.com")
x = /foo/
y = Regexp.new("bar")
z = URI.encode("baz")
::URI.parse("http://example.com")
