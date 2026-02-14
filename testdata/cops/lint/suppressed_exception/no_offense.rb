begin
  do_something
rescue => e
  handle(e)
end
begin
  work
rescue StandardError
  retry
end
