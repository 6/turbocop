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

if (arr[0])
   ^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a method call.
end

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

(foo.bar).to_s
^ Style/RedundantParentheses: Don't use parentheses around a method call.

(foo.bar(1)).to_json
^ Style/RedundantParentheses: Don't use parentheses around a method call.

(foo.bar).qux
^ Style/RedundantParentheses: Don't use parentheses around a method call.

(x.y).z(arg)
^ Style/RedundantParentheses: Don't use parentheses around a method call.

!(@groups.include?(g))
 ^ Style/RedundantParentheses: Don't use parentheses around a method call.

foo.include?((port = get_port))
             ^ Style/RedundantParentheses: Don't use parentheses around a method argument.

({filename: file, content: File.read(file)}.merge(opts)).to_json
^ Style/RedundantParentheses: Don't use parentheses around a method call.

({filename: file, content: File.read(file)}.merge(opts)).to_json
^ Style/RedundantParentheses: Don't use parentheses around a method call.

return ((isprint(c)) ? 1 : 2)
        ^ Style/RedundantParentheses: Don't use parentheses around a method call.

exit_code = (@codaveri_evaluation_results.map(&:success).all? { |n| n == 1 }) ? 0 : 2
            ^ Style/RedundantParentheses: Don't use parentheses around a method call.

new_file = [(Pod::Sandbox::PathList.new(@banana_spec.defined_in_file.dirname).root + 'CoolFile.h')]
            ^ Style/RedundantParentheses: Don't use parentheses around a method call.

c = (((c & 0x03ff)) << 10 | (low & 0x03ff)) + 0x10000
      ^ Style/RedundantParentheses: Don't use parentheses around a method call.

((1 << 128)).to_s(16), # 0
 ^ Style/RedundantParentheses: Don't use parentheses around a method call.

((1 << 64)).to_s(16),
 ^ Style/RedundantParentheses: Don't use parentheses around a method call.

match(event,
  (on Finished do
  ^^^^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a method argument.
     on_finish
   end),
)
