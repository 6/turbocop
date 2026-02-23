%i[foo bar baz]

[:foo]

[1, 2, 3]

[:foo, "bar"]

%i[one two]

[]

# Arrays with comments inside — %i[] can't contain comments
[
  :arg, :optarg, :restarg,
  :kwarg, :kwoptarg, :kwrestarg,
  :blockarg, # This doesn't mean block argument
  :shadowarg # This means block local variable
].freeze
