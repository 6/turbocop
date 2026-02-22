x = ("hello")
    ^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a literal.

x = (1)
    ^^^ Style/RedundantParentheses: Don't use parentheses around a literal.

x = (nil)
    ^^^^^ Style/RedundantParentheses: Don't use parentheses around a literal.

x = (self)
    ^^^^^^ Style/RedundantParentheses: Don't use parentheses around a keyword.

y = (a && b)
    ^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a logical expression.

return (foo.bar)
       ^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a method call.

x = (foo.bar)
    ^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a method call.

x = (foo.bar(1))
    ^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a method call.

(x == y)
^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a comparison expression.

(a >= b)
^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a comparison expression.

(x <=> y)
^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a comparison expression.

x =~ (%r{/\.{0,2}$})
     ^^^^^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a literal.

process((start..length), path, file)
        ^^^^^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a range literal.

x = (0..10)
    ^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a range literal.
