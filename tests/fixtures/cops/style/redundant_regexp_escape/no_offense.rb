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
# RuboCop keeps escaped hyphen immediately after `[^`
x =~ /[^\-^<]+/
# Escaping delimiter characters in %r(...) is not redundant
x =~ %r(\A[^\(]*time)i
x =~ %r(foo\(bar\))
x =~ %r{foo\{bar\}}
# Backslash-newline is a regexp line continuation, not a redundant escape
x =~ /a\
b/
# Line continuation inside a character class is also allowed
x =~ /[a\
b]/
# Real-world multiline token regexp from the corpus
BEG_REGEXP = /\G(?:\
(?# 1:  SPACE   )( +)|\
(?# 2:  NIL     )(NIL))/
# RuboCop only reports interpolated block-call regexps up to the first interpolation
rule %r{(#{complex_id})(#{ws}*)([\{\(])}mx do |m|
end
