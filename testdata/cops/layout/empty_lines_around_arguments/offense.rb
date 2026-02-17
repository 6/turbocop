# rblint-expect: 4:0 Layout/EmptyLinesAroundArguments: Empty line detected around arguments.
# rblint-expect: 11:0 Layout/EmptyLinesAroundArguments: Empty line detected around arguments.
# rblint-expect: 18:0 Layout/EmptyLinesAroundArguments: Empty line detected around arguments.
# rblint-expect: 25:0 Layout/EmptyLinesAroundArguments: Empty line detected around arguments.
# rblint-expect: 32:0 Layout/EmptyLinesAroundArguments: Empty line detected around arguments.
# rblint-expect: 37:0 Layout/EmptyLinesAroundArguments: Empty line detected around arguments.
# rblint-expect: 43:0 Layout/EmptyLinesAroundArguments: Empty line detected around arguments.
# rblint-expect: 45:0 Layout/EmptyLinesAroundArguments: Empty line detected around arguments.
# rblint-expect: 47:0 Layout/EmptyLinesAroundArguments: Empty line detected around arguments.
# Empty line between args
foo(
  bar,

  baz
)

# Empty line between args
something(
  first,

  second
)

# Empty line between first and rest
method_call(
  a,

  b,
  c
)

# Empty line before first arg
do_something(

  bar
)

# Empty line after last arg before closing paren
bar(
  [baz, qux]

)

# Args start on definition line with empty line
foo(biz,

    baz: 0)

# Multiple empty lines (3 offenses)
multi(
  baz,

  qux,

  biz,

)
