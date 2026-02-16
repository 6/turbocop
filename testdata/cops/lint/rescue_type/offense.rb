begin
  bar
rescue nil
^^^^^^ Lint/RescueType: Rescuing from `nil` will raise a `TypeError` instead of catching the actual exception.
  baz
end

begin
  bar
rescue 1
^^^^^^ Lint/RescueType: Rescuing from `1` will raise a `TypeError` instead of catching the actual exception.
  baz
end

begin
  bar
rescue 'a'
^^^^^^ Lint/RescueType: Rescuing from `'a'` will raise a `TypeError` instead of catching the actual exception.
  baz
end
