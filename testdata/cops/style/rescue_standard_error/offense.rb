begin
  foo
rescue StandardError
^^^^^^ Style/RescueStandardError: Omit the error class when rescuing `StandardError` by itself.
  bar
end

begin
  baz
rescue StandardError => e
^^^^^^ Style/RescueStandardError: Omit the error class when rescuing `StandardError` by itself.
  handle(e)
end

begin
  one
rescue StandardError
^^^^^^ Style/RescueStandardError: Omit the error class when rescuing `StandardError` by itself.
  two
end
