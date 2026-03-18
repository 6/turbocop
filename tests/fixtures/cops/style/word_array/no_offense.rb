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

# Matrix of complex content: parent array where all elements are arrays and
# at least one subarray has complex content (space in "United States").
# RuboCop exempts all subarrays in such a matrix.
[
  ["US", "United States"],
  ["UK", "United Kingdom"],
  ["CA", "Canada"]
]

# Matrix with all-word subarrays but one has a space
[
  ["AL", "Albania"],
  ["AS", "American Samoa"],
  ["AD", "Andorra"]
]

# Simple 2-element matrix where one pair has space
[["foo", "bar"], ["baz quux", "qux"]]

# Ambiguous block context: array arg to non-parenthesized method call with block.
# %w() would be ambiguous here (Ruby can't tell if { is a block or hash).
describe_pattern "LOG", ['legacy', 'ecs-v1'] do
  puts "test"
end

task :watch, ["account", "file-name"] do |t, args|
  puts args
end

describe ['module1', 'module2', 'module3'] do
  it { should be_in INSTALLED_MODULES }
end

# Parenthesized call with block is NOT ambiguous — this SHOULD fire,
# but the array is inside the parens so it's fine to flag.
# (This test ensures we only suppress non-parenthesized calls.)
