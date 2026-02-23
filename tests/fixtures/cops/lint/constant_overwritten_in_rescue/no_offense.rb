begin
  something
rescue StandardError
  handle_error
end

begin
  something
rescue StandardError => e
  handle_error(e)
end

begin
  something
rescue => e
  handle_error(e)
end

# rescue => Constant with a body is intentional (capturing to constant on purpose)
handler = lambda do
  begin
    something
  rescue => CapturedError
    log(CapturedError)
  end
end
