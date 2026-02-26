foo(
  bar,
  baz
)

something(first, second, third)

method_call(
  arg1,
  arg2
)

# Chained method calls where receiver is on a different line
Foo
  .new(label: false,
       number_length_limit: 42)

render Primer::Beta::Counter
  .new(count:,
       hide_if_zero: true)

something
  .call(foo,
        bar)

# All arguments on the same line as call, closing paren on next line
raise_error(SomeError, %r(some message)
                 )
