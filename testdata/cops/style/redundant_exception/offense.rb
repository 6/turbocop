raise RuntimeError, "message"
^^^^^ Style/RedundantException: Redundant `RuntimeError` argument can be removed.

fail RuntimeError, "message"
^^^^ Style/RedundantException: Redundant `RuntimeError` argument can be removed.

raise RuntimeError.new("message")
^^^^^ Style/RedundantException: Redundant `RuntimeError.new` call can be replaced with just the message.
