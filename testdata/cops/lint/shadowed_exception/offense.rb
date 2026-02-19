begin
  something
rescue Exception
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  handle_exception
rescue StandardError
  handle_standard_error
end

begin
  something
rescue Exception
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  handle_exception
rescue NoMethodError, ZeroDivisionError
  handle_standard_error
end

begin
  something
rescue Exception, StandardError
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  handle_error
end

# Standard library: IPAddr::InvalidAddressError < IPAddr::Error
begin
  IPAddr.new(uri.host).loopback?
rescue IPAddr::Error, IPAddr::InvalidAddressError
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  false
end
