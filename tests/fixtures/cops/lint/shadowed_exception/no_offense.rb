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

# Psych subclasses are siblings under Psych::Exception, not nested under SyntaxError
begin
  parse_config
rescue Psych::SyntaxError, Psych::DisallowedClass, Psych::BadAlias => e
  warn e.message
end

begin
  parse_config
rescue Errno::ENOENT
  warn "missing"
rescue Psych::SyntaxError => e
  warn e.message
rescue Psych::BadAlias => e
  warn e.message
end

# Two consecutive bare rescue clauses — not shadowed (same implicit StandardError)
begin
  event
rescue
  fallback_one
rescue
  fallback_two
end

# Unknown exception classes not in hierarchy — don't flag as shadowed
begin
  if user.nickname_changed?
    user.save!
  end
rescue ActiveRecord::RecordInvalid, ActiveRecord::RecordNotUnique
  retry_with_new_nickname(user)
rescue ActiveRecord::RecordInvalid
  logger.warn("failed")
end

# Unresolved third-party duplicates should not be treated as equivalent
begin
  yield
rescue Stripe::CardError,
       Stripe::InvalidRequestError,
       Stripe::RateLimitError,
       Stripe::InvalidRequestError,
       Stripe::AuthenticationError,
       Stripe::APIConnectionError,
       Stripe::StripeError => e
  StripeLogger.error(e.json_body.try(:[], :error))
end
