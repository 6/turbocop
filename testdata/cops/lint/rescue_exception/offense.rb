begin
  do_something
rescue Exception
       ^^^^^^^^^ Lint/RescueException: Avoid rescuing the `Exception` class. Perhaps you meant `StandardError`?
  handle_error
end

begin
  foo
rescue Exception => e
       ^^^^^^^^^ Lint/RescueException: Avoid rescuing the `Exception` class. Perhaps you meant `StandardError`?
  log(e)
end

begin
  bar
rescue RuntimeError, Exception
                     ^^^^^^^^^ Lint/RescueException: Avoid rescuing the `Exception` class. Perhaps you meant `StandardError`?
  baz
end
