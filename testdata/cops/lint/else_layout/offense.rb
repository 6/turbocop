if foo
  one
else bar
     ^^^ Lint/ElseLayout: Odd `else` layout detected. Code on the same line as `else` is not allowed.
end
if baz
  two
else qux
     ^^^ Lint/ElseLayout: Odd `else` layout detected. Code on the same line as `else` is not allowed.
end
if alpha
  three
else beta
     ^^^^ Lint/ElseLayout: Odd `else` layout detected. Code on the same line as `else` is not allowed.
end
