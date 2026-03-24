begin
  foo
rescue => e
  bar
end
begin
  foo
rescue StandardError => e
  bar
end
begin
  foo
rescue => e
  bar
end
begin
  something
rescue => @exception
end
begin
  something
rescue => @@captured_error
end
begin
  something
rescue => $error
end

# Writing to the preferred name in the body is NOT shadowing (only reads count)
begin
  do_something
rescue RuntimeError => e
  e = e
end

# ConstantPathTargetNode (qualified constant as rescue variable)
module M
end
begin
  raise 'foo'
rescue => M::E
end

# Top-level ConstantPathTargetNode
begin
  raise 'foo'
rescue => ::E2
end

# Method-body rescue (no explicit begin)
def process
  do_work
rescue RuntimeError => e
  handle(e)
end

# Underscore-prefixed variable should suggest _e
begin
  something
rescue MyException => _e
  # ignored
end

# Multiple exception types (comma-separated) with bad variable name
begin
  something
rescue ArgumentError, TypeError => e
  handle(e)
end

# Multiple rescues in same begin block
begin
  something
rescue FooException => e
  # handle foo
rescue BarException => e
  # handle bar
end
