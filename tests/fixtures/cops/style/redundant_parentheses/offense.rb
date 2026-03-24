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

(-> { x })
^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around an expression.

(lambda { x })
^^^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around an expression.

(proc { x })
^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around an expression.

(defined?(:A))
^^^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a keyword.

(yield)
^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a keyword.

(yield())
^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a keyword.

(yield(1, 2))
^^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a keyword.

(super)
^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a keyword.

(super())
^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a keyword.

(super(1, 2))
^^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a keyword.

(x === y)
^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a comparison expression.

x.y((z))
    ^^^ Style/RedundantParentheses: Don't use parentheses around a method argument.

x.y((z + w))
    ^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a method argument.

x&.y((z))
     ^^^ Style/RedundantParentheses: Don't use parentheses around a method argument.

x.y(a, (b))
       ^^^ Style/RedundantParentheses: Don't use parentheses around a method argument.

return (foo + bar)
       ^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a method call.

(foo rescue bar)
^^^^^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a one-line rescue.

return (42)
       ^^^^ Style/RedundantParentheses: Don't use parentheses around a literal.

(!x arg)
^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a unary operation.

(!x.m arg)
^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a unary operation.

x.y((a..b))
    ^^^^^^ Style/RedundantParentheses: Don't use parentheses around a method argument.

x.y((1..42))
    ^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a method argument.

"#{(foo)}"
   ^^^^^ Style/RedundantParentheses: Don't use parentheses around an interpolated expression.

(expression in pattern)
^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a one-line pattern matching.

(expression => pattern)
^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a one-line pattern matching.
