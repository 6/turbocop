foo(
  bar, baz,
       ^^^ Layout/MultilineMethodArgumentLineBreaks: Each argument in a multi-line method call must start on a separate line.
  qux
)

something(
  first, second,
         ^^^^^^ Layout/MultilineMethodArgumentLineBreaks: Each argument in a multi-line method call must start on a separate line.
  third
)

method_call(
  a, b,
     ^ Layout/MultilineMethodArgumentLineBreaks: Each argument in a multi-line method call must start on a separate line.
  c
)
