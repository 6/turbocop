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
