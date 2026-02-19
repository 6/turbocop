begin
  do_something
rescue
  handle_error
end

begin
  something
rescue => e
  handle_error
else
  success
ensure
  cleanup
end

def foo
  bar
rescue
  baz
end

x = <<~RUBY
  begin
    something

  rescue

    handle
  end
RUBY

NODES = %i[if while rescue ensure else].freeze

# else in if/case is NOT exception handling — should not be flagged
if condition

else
  handle
end

case x

else
  default
end
