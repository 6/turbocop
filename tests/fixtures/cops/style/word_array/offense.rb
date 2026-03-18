['foo', 'bar', 'baz']
^ Style/WordArray: Use `%w` or `%W` for an array of words.

['one', 'two']
^ Style/WordArray: Use `%w` or `%W` for an array of words.

x = ['alpha', 'beta', 'gamma']
    ^ Style/WordArray: Use `%w` or `%W` for an array of words.

# Hyphenated words should be flagged (matches default WordRegex)
['foo', 'bar', 'foo-bar']
^ Style/WordArray: Use `%w` or `%W` for an array of words.

# Unicode word characters should be flagged
["hello", "world", "caf\u00e9"]
^ Style/WordArray: Use `%w` or `%W` for an array of words.

# Strings with newline/tab escapes are words per default WordRegex
["one\n", "hi\tthere"]
^ Style/WordArray: Use `%w` or `%W` for an array of words.

# Matrix where all subarrays are simple words — each subarray still flagged
[
  ["one", "two"],
  ^ Style/WordArray: Use `%w` or `%W` for an array of words.
  ["three", "four"]
  ^ Style/WordArray: Use `%w` or `%W` for an array of words.
]

# Parenthesized call with block is NOT ambiguous — should still flag
foo(['bar', 'baz']) { qux }
    ^ Style/WordArray: Use `%w` or `%W` for an array of words.

# Matrix with mixed-type subarrays — pure-string subarrays still flagged
# (non-string elements like 0 don't make a subarray "complex" for matrix check)
[["foo", "bar", 0], ["baz", "qux"]]
                    ^ Style/WordArray: Use `%w` or `%W` for an array of words.
