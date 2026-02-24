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
