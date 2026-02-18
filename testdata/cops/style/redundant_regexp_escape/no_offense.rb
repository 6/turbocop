x =~ /\./
x =~ /\d+/
x =~ /\[foo\]/
x =~ /\\/
x =~ /foo/
y = 'hello'
# Escape hyphen in the middle of char class is meaningful
x =~ /[\s\-a]/
# Escape sequences in char class are meaningful
x =~ /[\w\d\s]/
# Escape bracket inside char class is meaningful
x =~ /[\]]/
# Escaping delimiter characters in %r(...) is not redundant
x =~ %r(\A[^\(]*time)i
x =~ %r(foo\(bar\))
x =~ %r{foo\{bar\}}
