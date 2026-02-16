begin
  foo
rescue => e
  bar(e)
end
begin
  foo
rescue StandardError => e
  bar(e)
end
begin
  foo
rescue
  bar
end
