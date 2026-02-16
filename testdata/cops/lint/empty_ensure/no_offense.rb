begin
  do_something
rescue
  handle_error
ensure
  cleanup
end

begin
  foo
ensure
  bar
  baz
end
