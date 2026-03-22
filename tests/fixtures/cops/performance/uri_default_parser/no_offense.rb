URI::DEFAULT_PARSER.make_regexp
URI::DEFAULT_PARSER.unescape('foo')
URI::DEFAULT_PARSER.escape('bar')
URI.parse('http://example.com')
::URI.parse('http://example.com')
URI.encode('foo')
URI.decode('foo')
::URI.encode('foo')
::URI.decode('foo')
obj.new
Addressable::URI::Parser.new
Something::Parser.new
# URI::Parser.new with arguments creates a custom parser, not DEFAULT_PARSER
URI::Parser.new(key: value)
URI::Parser.new(:UNRESERVED => "\\-_.!~*'()a-zA-Z\\d" + "|")
::URI::Parser.new(:UNRESERVED => URI::REGEXP::PATTERN::UNRESERVED + '|')
