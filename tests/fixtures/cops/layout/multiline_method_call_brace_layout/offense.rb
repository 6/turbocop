foo(a,
  b
)
^ Layout/MultilineMethodCallBraceLayout: Closing method call brace must be on the same line as the last argument when opening brace is on the same line as the first argument.

bar(
  a,
  b)
   ^ Layout/MultilineMethodCallBraceLayout: Closing method call brace must be on the line after the last argument when opening brace is on a separate line from the first argument.

baz(c,
  d
)
^ Layout/MultilineMethodCallBraceLayout: Closing method call brace must be on the same line as the last argument when opening brace is on the same line as the first argument.
