begin
  bar
rescue
  baz
end

begin
  bar
rescue NameError
  baz
end

begin
  bar
rescue StandardError => e
  baz
end
