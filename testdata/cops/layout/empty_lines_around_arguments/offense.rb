# rblint-expect: 3:0 Layout/EmptyLinesAroundArguments: Extra empty line detected inside method arguments.
# rblint-expect: 9:0 Layout/EmptyLinesAroundArguments: Extra empty line detected inside method arguments.
# rblint-expect: 15:0 Layout/EmptyLinesAroundArguments: Extra empty line detected inside method arguments.
foo(
  bar,

  baz
)

something(
  first,

  second
)

method_call(
  a,

  b,
  c
)
