r = /[xy]/
r = /[abc]/
r = /[0-9]/
r = /foo/
r = /[a-z]/
# POSIX character classes should not trigger false positives
r = /[[:digit:][:upper:]_]+/
r = /[[:alnum:]:]/
r = /[[:alpha:][:digit:]]+/
r = /\(#([[:digit:]]+)\)/
# Unicode property escapes should not trigger false positives
r = /[^\p{White_Space}<>()?]/
r = /[\p{L}\p{N}_-]/
r = /[^\P{ASCII}]/
# Character class intersection should not trigger false positives
r = /[\S&&[^\\]]/
r = /[a-z&&[^aeiou]]/
# Hex escape sequences in character classes should not trigger false positives
r = /[\x00-\x1F\x7F]/
r = /[\x20-\x7E]/
r = /[\\<>@"!#$%&*+=?^`{|}~:;]/
# Octal escapes should not trigger false positives
r = /[\0\1\2]/
# Escaped backslash before bracket should not be treated as escaped bracket
r = /(\\[[:space:]]|[^[:space:]])*/
# POSIX class in negated character class is not a duplicate
r = /[^[:space:]]/
# Escaped brackets are literal, not character class delimiters
r = /(\w+\.|\[\w+\]\.)?/
# Mixed escaped brackets and character classes
r = /(?:\w+|\[\w+\])/
# Extended mode (/x) comments should not be treated as regex content
r = /
  [a-z]          # matches [lowercase] letters
  [0-9]          # matches [digits]
/x
r = /
  "([^"]+)"      # capture "quoted" text
  \s+            # whitespace
/x
r = /
  (\w+)          # word chars
  (["']).+?\1    # quoted string with ["'] chars
/x
# Nested character classes should be treated as grouped elements, not duplicate brackets
r = /[[a-c][x-z][0-2]]+/
r = /[^a-c[x-z][0-2]]+/
r = /[a-c[x-z[^0-2]]]+/
# Nested POSIX classes inside larger sets should not trigger false positives
BAD_SHIFT_REGEX = /\[\[([[[:alpha:]][[:blank:]]|,\(\)\-[[:digit:]]]+)\}\}/
r = /\A([[[:upper:]][[:punct:]]] )+[[[:upper:]][[:punct:]]]?$\z/
r = /(\\|^)[[:upper:]][[[:upper:]][[:digit:]]_]+$/
r = /^@{0,2}(?:_|_?[[[:lower:]]][\d[[:lower:]][[:upper:]]]*)[!?=]?$/
