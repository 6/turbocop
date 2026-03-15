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

# LoadError and SyntaxError are ScriptError subclasses, not StandardError
begin
  something
rescue StandardError, SyntaxError, LoadError => e
  handle_error(e)
end

# LoadError, StandardError in same rescue (different hierarchy branches)
begin
  something
rescue LoadError, StandardError
  handle_error
end

# Net::ProtocolError with Net::HTTPBadResponse — not in RuboCop's known hierarchy
begin
  something
rescue SocketError, Timeout::Error, Errno::EINVAL, Errno::ECONNRESET,
       EOFError, Net::HTTPBadResponse, Net::HTTPHeaderSyntaxError,
       Net::ProtocolError, RestClient::ResourceNotFound => e
  handle_error(e)
end
