[1, 2, 3]

[1]

[]

["a", "b"]

[:foo, :bar]

# Word/symbol arrays don't use commas — never flagged
%w(
  foo
  bar
)

%i(foo bar baz)

%W[one two three]

# Multiline array with single element — closing bracket on same line as element
# (allowed_multiline_argument pattern — not flagged under no_comma default)
[
  some_method_call(
    arg1, arg2
  )]

# Multiline array that already has no trailing comma (no_comma style)
[
  1,
  2,
  3
]

# Array containing a heredoc element (heredoc content has commas, not array commas)
[
  <<~OUTPUT.chomp
    The `Style/PredicateName` cop has been renamed, please update it
  OUTPUT
]

# Single-line array with heredoc element — no trailing comma
cmd = ['-W0', '-e', <<-RB]
  puts 'foo'
  print 'bar'
RB

# Single-line array with multiple heredoc elements
x = [<<EOS1, <<EOS2]
first content
EOS1
second content
EOS2
