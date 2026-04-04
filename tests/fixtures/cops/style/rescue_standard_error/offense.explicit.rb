begin
  do_something
rescue
^^^^^^ Style/RescueStandardError: Specify `StandardError` explicitly when rescuing.
  handle_error
end

begin
  do_something
rescue => e
^^^^^^ Style/RescueStandardError: Specify `StandardError` explicitly when rescuing.
  handle_error(e)
end
