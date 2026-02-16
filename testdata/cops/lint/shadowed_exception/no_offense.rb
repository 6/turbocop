begin
  something
rescue StandardError
  handle_standard_error
rescue Exception
  handle_exception
end

begin
  something
rescue ArgumentError
  handle_argument_error
rescue StandardError
  handle_standard_error
end

begin
  something
rescue RuntimeError
  handle_runtime
rescue StandardError
  handle_standard
rescue Exception
  handle_exception
end
