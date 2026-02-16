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
