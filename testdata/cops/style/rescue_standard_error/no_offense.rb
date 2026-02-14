begin
  foo
rescue
  bar
end

begin
  foo
rescue RuntimeError
  bar
end

begin
  foo
rescue StandardError, RuntimeError
  bar
end

begin
  foo
rescue ArgumentError => e
  bar
end
