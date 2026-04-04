begin
  do_something
rescue StandardError
  handle_error
end

begin
  do_something
rescue StandardError => e
  handle_error(e)
end

begin
  do_something
rescue RuntimeError
  handle_error
end

begin
  do_something
rescue ArgumentError, TypeError => e
  handle_error(e)
end
