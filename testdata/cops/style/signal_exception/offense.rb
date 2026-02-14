fail RuntimeError, "message"
^^^^ Style/SignalException: Use `raise` instead of `fail` to rethrow exceptions.

fail "something went wrong"
^^^^ Style/SignalException: Use `raise` instead of `fail` to rethrow exceptions.

fail ArgumentError, "bad argument"
^^^^ Style/SignalException: Use `raise` instead of `fail` to rethrow exceptions.
