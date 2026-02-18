do_something(foo: bar, baz: qux)
method(**options)
call(a: 1, **opts)
do_something(**variable)
foo(bar: baz)
method(**config)

# Empty hash splat is valid
do_something(**{})

# Double splat in hash literal (not a method call)
h = { **{ a: 1 } }

# Hash rocket syntax - can't convert to keyword args (dynamic keys)
do_something(**{ variable => value })
assert_enqueued_with(**{ facet => matcher })
