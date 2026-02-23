p /pattern/
  ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
p /pattern/, foo
  ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
puts line.grep /pattern/
               ^ Lint/AmbiguousRegexpLiteral: Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.
