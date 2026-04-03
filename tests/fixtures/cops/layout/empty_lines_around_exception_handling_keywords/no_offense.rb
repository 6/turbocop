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

begin
  install_ri
rescue NameError; nil end

begin
  install_ri

rescue NameError; nil end

def foo
  work rescue nil

  other
end

def multi_statement_method
  first

  work rescue nil
end

foo do
  work rescue nil

  other
end

foo do
  first

  work rescue nil
end

def install_rdoc
  install_rdoc_yard
end

class C
  a
  b

rescue StandardError => e
  handle_error
end

module M
  a
  b

rescue StandardError => e
  handle_error
end

=begin
begin
  work

rescue => e
  handle
end
=end
