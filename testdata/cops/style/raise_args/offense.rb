raise RuntimeError.new("message")
^^^^^ Style/RaiseArgs: Provide an exception class and message as separate arguments.

raise ArgumentError.new("bad argument")
^^^^^ Style/RaiseArgs: Provide an exception class and message as separate arguments.

fail StandardError.new("oops")
^^^^ Style/RaiseArgs: Provide an exception class and message as separate arguments.
