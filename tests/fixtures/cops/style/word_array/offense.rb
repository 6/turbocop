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

# %w with backslash-escaped space — single line, various styles
%w(Cucumber\ features features)
^ Style/WordArray: Use `['Cucumber features', 'features']` for an array of words.

%w[hello\ world foo]
^ Style/WordArray: Use `['hello world', 'foo']` for an array of words.

# %W with backslash-escaped space — multi-line
x = %W(
    ^ Style/WordArray: Use an array literal `[...]` for an array of words.
  normal
  hello\ world
)

# %w with backslash-escaped space — multi-line
y = %w(
    ^ Style/WordArray: Use an array literal `[...]` for an array of words.
  hello\ world
  foo
)

# Nested word arrays inside a complex matrix should still be flagged.
options = [
  ["North America", [["United States", "US"], "Canada"]],
  ["Europe", ["Denmark", "Germany", "France"]]
             ^ Style/WordArray: Use `%w` or `%W` for an array of words.
]

LANGUAGE_ARRAY = [
  ["Bahasa Indonesia", "id", ["id-ID"]],
  ["বাংলা", "bn", ["bn-BD", "বাংলাদেশ"]]
                  ^ Style/WordArray: Use `%w` or `%W` for an array of words.
]
