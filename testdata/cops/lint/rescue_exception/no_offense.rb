begin
  do_something
rescue StandardError
  handle_error
end

begin
  foo
rescue RuntimeError, IOError
  bar
end

begin
  baz
rescue => e
  log(e)
end
