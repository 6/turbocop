p /pattern/
  ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
p /pattern/, foo
  ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
puts line.grep /pattern/
               ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
p /pattern/.do_something
  ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
p /pattern/.do_something(42)
  ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
p /pattern/.do_something.do_something
  ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
class MyTest
  test '#foo' do
    assert_match /expected/, actual
                 ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
  end
end
expect('RuboCop').to(match /Cop/)
                           ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
expect('RuboCop').to match /Robo/
                           ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
assert /some pattern/ =~ some_string
       ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
p /pattern/ do
  ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
  p /pattern/
    ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
end
p /pattern/, foo do |arg|
  ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
end
