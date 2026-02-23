# Exploded style (default): flag raise/fail with Error.new(single_arg)
raise RuntimeError.new("message")
^^^^^ Style/RaiseArgs: Provide an exception class and message as arguments to `raise`.

raise ArgumentError.new("bad argument")
^^^^^ Style/RaiseArgs: Provide an exception class and message as arguments to `raise`.

fail StandardError.new("oops")
^^^^ Style/RaiseArgs: Provide an exception class and message as arguments to `fail`.

raise RuntimeError.new
^^^^^ Style/RaiseArgs: Provide an exception class and message as arguments to `raise`.

raise Foo::Bar.new("message")
^^^^^ Style/RaiseArgs: Provide an exception class and message as arguments to `raise`.
