URI::DEFAULT_PARSER.unescape('foo')
URI::DEFAULT_PARSER.escape('bar')
URI.parse('http://example.com')
obj.decode('foo')
CGI.escape('bar')
::URI.parse('http://example.com')
# Namespaced URI classes should not be flagged
Addressable::URI.encode(uri)
Addressable::URI.decode(uri)
