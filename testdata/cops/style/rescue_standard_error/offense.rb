begin
  foo
rescue StandardError
^^^^^^ Style/RescueStandardError: Omit the error class when rescuing `StandardError` by itself.
  bar
end
