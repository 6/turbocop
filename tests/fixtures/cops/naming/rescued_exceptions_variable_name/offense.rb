begin
  foo
rescue => ex
          ^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `ex` for rescued exceptions.
  bar
end
begin
  foo
rescue StandardError => err
                        ^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `err` for rescued exceptions.
  bar
end
begin
  foo
rescue => exception
          ^^^^^^^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `exception` for rescued exceptions.
  bar
end
begin
  something
rescue => @exception
          ^^^^^^^^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `@exception` for rescued exceptions.
end
begin
  something
rescue => @@captured_error
          ^^^^^^^^^^^^^^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `@@captured_error` for rescued exceptions.
end
begin
  something
rescue => $error
          ^^^^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `$error` for rescued exceptions.
end

# Writing to the preferred name in the body is NOT shadowing (only reads count)
begin
  do_something
rescue RuntimeError => error
                       ^^^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `error` for rescued exceptions.
  e = error
end

# ConstantPathTargetNode (qualified constant as rescue variable)
module M
end
begin
  raise 'foo'
rescue => M::E
          ^^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `M::E` for rescued exceptions.
end

# Top-level ConstantPathTargetNode
begin
  raise 'foo'
rescue => ::E2
          ^^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `::E2` for rescued exceptions.
end

# Method-body rescue (no explicit begin)
def process
  do_work
rescue RuntimeError => exc
                       ^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `exc` for rescued exceptions.
  handle(exc)
end

# Underscore-prefixed variable should suggest _e
begin
  something
rescue MyException => _exc
                      ^^^^ Naming/RescuedExceptionsVariableName: Use `_e` instead of `_exc` for rescued exceptions.
  # ignored
end

# Multiple rescues in same begin block
begin
  something
rescue FooException => foo
                       ^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `foo` for rescued exceptions.
  # handle foo
rescue BarException => bar
                       ^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `bar` for rescued exceptions.
  # handle bar
end
