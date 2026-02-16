begin
  do_something
rescue
  handle_error
end

begin
  something
rescue => e
  handle_error
else
  success
ensure
  cleanup
end

def foo
  bar
rescue
  baz
end
