begin
  do_something
rescue ArgumentError
  handle_arg
rescue RuntimeError
  handle_runtime
end

begin
  foo
rescue IOError, Errno::ENOENT
  bar
end
