%w[foo bar baz]

['foo']

['foo bar', 'baz']

[1, 2, 3]

%w[one two]

[]

# Array with empty string — can't be represented in %w
['es', 'fr', '']

# Single-quoted string with literal backslash — not a word
['has\nescapes', 'foo']

# Non-word strings (hyphens only, not connecting word chars)
['-', '----']

# Array with comments inside
[
"foo", # a comment
"bar", # another comment
"baz"  # trailing comment
]

# Array with space in a string
["one space", "two", "three"]
