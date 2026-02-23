# Exploded style (default): these are all fine
raise RuntimeError, "message"

raise "something went wrong"

raise

fail ArgumentError, "bad"

raise StandardError

obj.raise RuntimeError.new("ok")

# Multi-arg .new is allowed (can't convert to exploded form)
raise MyError.new(arg1, arg2)

raise MyError.new(arg1, arg2, arg3)

# Hash arg to .new is allowed
raise MyError.new(key: "value")

# Splat arg to .new is allowed
raise MyError.new(*args)

# Double-splat (forwarding hash) is allowed
raise MyError.new(**opts)
