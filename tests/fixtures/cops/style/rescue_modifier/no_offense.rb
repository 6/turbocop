begin
  something
rescue StandardError => e
  handle(e)
end

begin
  foo
rescue ArgumentError
  bar
end

begin
  foo
rescue RuntimeError, StandardError
  bar
end
