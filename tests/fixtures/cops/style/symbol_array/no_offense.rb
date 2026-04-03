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

# Symbol arrays as arguments to non-parenthesized method calls with blocks
# (invalid_percent_array_context? — %i is ambiguous in this position)
can [:admin, :read, :index, :update, :destroy], Product do |product|
end

can [:admin, :create, :update], Item do |item|
end

foo [:one, :two, :three] { |x| x }

# Symbol containing spaces — complex content that %i can't represent
[:"foo bar", :baz, :qux]

# Symbol containing unclosed delimiters
[:one, :")", :two, :"(", :"]"]

# Symbol containing delimiter with spaces inside
[:one, :two, :"[ ]", :"( )"]

# Arrays containing an empty quoted symbol — RuboCop does not treat `:""` as a plain `sym`
[:foo, :""]
