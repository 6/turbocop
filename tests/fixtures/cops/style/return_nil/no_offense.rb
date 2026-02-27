def foo
  return
end

def bar
  return value
end

def baz
  return 1, 2
end
x = 1
y = 2

# return nil inside iterator blocks should NOT be flagged
# (defers to Lint/NonLocalExitFromIterator)
def method_with_each
  items.each do |item|
    return nil if item.nil?
  end
end

def method_with_map
  items.map do |item|
    return nil unless item.valid?
  end
end

def method_with_each_with_object
  items.each_with_object({}) do |item, hash|
    return nil unless valid?(item)
  end
end

# Bare block (no receiver) — return nil IS flagged
# (not an iterator, so no non-local exit concern)
# This is NOT a no_offense case; it should still be flagged.

# But define_method blocks should NOT suppress
# (define_method creates a proper scope, so return nil IS flagged)
# This is NOT a no_offense case; it should still be flagged.

# Nested: return nil inside iterator inside def should not be flagged
def nested_example
  tokens.each do |token|
    next if token.empty?
    return nil if token == "stop"
  end
end
