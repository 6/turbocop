begin
  foo
rescue => e
  bar
end

begin
  foo
ensure
  bar
end

def baz
  foo
rescue => e
  bar
end
