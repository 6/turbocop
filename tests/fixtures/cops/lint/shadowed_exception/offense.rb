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

# Timeout::Error shadows Net::OpenTimeout and Net::ReadTimeout
begin
  something
rescue Net::OpenTimeout, Net::ReadTimeout, Timeout::Error, SocketError => e
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  handle_error(e)
end

# StandardError shadows Timeout::Error (Timeout::Error < StandardError)
begin
  something
rescue StandardError, Timeout::Error => e
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  handle_error(e)
end

# Errno::EPIPE < SystemCallError — mixed levels in single rescue
begin
  something
rescue Errno::EPIPE, SystemCallError, IOError
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  handle_error
end

# OpenSSL::PKey::PKeyError shadows RSAError, DSAError, ECError
begin
  something
rescue OpenSSL::PKey::RSAError, OpenSSL::PKey::DSAError, OpenSSL::PKey::ECError, OpenSSL::PKey::PKeyError => e
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  handle_error(e)
end

# Zlib::Error shadows Zlib::GzipFile::Error
begin
  something
rescue Zlib::GzipFile::Error, Zlib::Error => e
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  handle_error(e)
end

# Date::Error < ArgumentError
begin
  something
rescue Date::Error, ArgumentError
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  handle_error
end

# Timeout::Error, StandardError — reversed order, still shadowed
begin
  something
rescue Timeout::Error, StandardError => e
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  handle_error(e)
end
