begin
  do_something
rescue StandardError
  handle_error
end

begin
  foo
rescue RuntimeError, IOError
  bar
end

begin
  baz
rescue => e
  log(e)
end

# Qualified Exception classes are NOT the top-level Exception
begin
  foo
rescue Gem::Exception
  bar
end

begin
  foo
rescue Gem::Security::Exception
  bar
end

begin
  foo
rescue YAMLSchema::Validator::Exception => e
  bar(e)
end
