top.test()
        ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

foo.bar()
       ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

obj&.baz()
        ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

# it() with receiver is flagged
0.times { foo.it() }
                ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

# it() in def body is flagged
def foo
  it()
    ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.
end

# it() in block with explicit empty params is flagged
0.times { ||
  it()
    ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.
}

# it() in block with named params is flagged
0.times { |_n|
  it()
    ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.
}

# Same-name assignment with receiver is still flagged
test = x.test()
             ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

# obj.method ||= func() — the func() is flagged
obj.method ||= func()
                   ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

# obj.method += func() — the func() is flagged
obj.method += func()
                  ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

# Mass assignment where LHS is a send (c[2]) — method with same name is flagged
c[2], x = c()
           ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

at_exit() do
       ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

synchronize() do
           ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

synchronize() do
           ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

synchronize_allow_reads() do
                       ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

synchronize_allow_reads() do
                       ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

synchronize() do
           ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

synchronize() do
           ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.

synchronize() do
           ^^ Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method calls with no arguments.
