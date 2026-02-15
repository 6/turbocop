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
