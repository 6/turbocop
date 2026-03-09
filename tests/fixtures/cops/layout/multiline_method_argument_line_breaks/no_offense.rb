foo(
  bar,
  baz,
  qux
)

something(first, second, third)

method_call(
  a,
  b,
  c
)

# All args on same line in multiline call (all_on_same_line? early return)
taz(
  "abc", "foo"
)

# Single keyword hash arg should not trigger
render(
  status: :ok,
  json: payload
)

# Bracket assignment should be skipped
bar['foo'] = ::Time.zone.at(
               huh['foo'],
             )

# Bracket assignment with multiple args on same line
a['b',
    'c', 'd'] = e

# Safe navigation with single arg
foo&.bar(baz)

# Safe navigation with all args on one line
foo&.bar(baz, quux)

# No-parens command call on one line
render json: data, status: :ok

# No-parens command call each arg on separate line
render json: data,
       status: :ok,
       layout: false

# No-parens command call with single keyword arg (no expansion needed)
render status: :ok
