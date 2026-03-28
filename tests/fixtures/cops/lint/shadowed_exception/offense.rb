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

# Leading :: still refers to the same built-in exception constant
begin
  do_work
rescue ::Exception, Timeout::Error => ex
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  warn ex.message
end

# Leading :: on nested constants should still participate in hierarchy checks
begin
  parse_config
rescue StandardError, ::Psych::SyntaxError => error
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  warn error.message
end

# OpenSSL::PKey error constants are aliases of the same underlying class
begin
  load_key
rescue OpenSSL::PKey::RSAError, OpenSSL::PKey::DSAError => e
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  warn e.message
end

# Known duplicate built-in exceptions should still be flagged
begin
  load_value
rescue NameError, NameError
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  nil
end

def fetch_meetup_token(response, provider)
  begin
    raise StandardError unless response.status == 200
    data = JSON.parse(response.body)
  rescue JSON::ParserError, StandardError => e
  ^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
    log(:error, "Failed to retrieve access token for #{provider.name}")
    return false
  end

  provider.token = data["access_token"]
end

def fetch_outlook_token(response, provider)
  begin
    raise StandardError unless response.status == 200
    data = JSON.parse(response.body)
  rescue JSON::ParserError, StandardError => e
  ^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
    log(:error, "Failed to retrieve access token for #{provider.name}")
    return false
  end

  provider.token = data["access_token"]
end

def write_highlighted_diff(redis, hash, key)
  redis.pipelined do |pipeline|
    hash.each do |diff_file_id, highlighted_diff_lines_hash|
      pipeline.hset(
        key,
        diff_file_id,
        gzip_compress(highlighted_diff_lines_hash.to_json)
      )
    rescue Encoding::UndefinedConversionError, EncodingError, JSON::GeneratorError
    ^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
      nil
    end

    pipeline.expire(key, EXPIRATION)
  end
end

def adapter_load(string, *args, **opts)
  opts = standardize_opts(opts)

  Oj.load(string, opts)
rescue Oj::ParseError, EncodingError, Encoding::UndefinedConversionError, JSON::GeneratorError => ex
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  raise parser_error, ex
end

def handle_shippo_response
  begin
    parse_response!
  rescue ::RestClient::BadRequest => e
    raise Shippo::Exceptions::APIServerError.new("bad request", self, e.response, e.message)
  rescue ::JSON::JSONError, ::JSON::ParserError => e
  ^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
    raise Shippo::Exceptions::InvalidJsonError.new(e.message)
  rescue ::RestClient::Exception => e
    raise Shippo::Exceptions::ConnectionError.new(connection_error_message(url, e))
  end
end

def self.handle_api_error(rcode, rbody)
  begin
    error_obj = JSON.parse(rbody)
    error = error_obj[:error] or raise StandardError.new
  rescue JSON::ParserError, StandardError
  ^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
    raise general_api_error(rcode, rbody)
  end

  error
end

def on_ready
  retries = 3

  begin
    Toxiproxy.populate(proxies)
  rescue SystemCallError, Net::HTTPError, Net::ProtocolError
  ^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
    retries -= 1
    retry if retries > 0
  end
end

def request_id
  begin
    body = request.body.read
    request.body.rewind
    json = JSON.parse(body)
    json["id"]
  rescue JSON::ParserError, StandardError
  ^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
    nil
  end
end

def lookup(domain, dnsbl, iplookup = false)
  Timeout::timeout(1) do
    domain = IPSocket::getaddress(domain).split(/\./).reverse.join(".") if iplookup
    address = Resolv.getaddress("#{domain}.#{dnsbl}")
    return true
  end
rescue Timeout::Error, Resolv::ResolvTimeout
^^^^^^ Lint/ShadowedException: Do not shadow rescued Exceptions.
  return false
rescue Resolv::ResolvError, Exception
  return false
end
