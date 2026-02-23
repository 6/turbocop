begin
  do_something
rescue
  handle_error
end

begin
  something
ensure
  cleanup
end
