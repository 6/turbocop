class Foo
  attr_accessor :foo

  def do_something
  end
end

class Bar
  attr_accessor :foo
  attr_reader :bar
  attr_writer :baz

  def example
  end
end

class Baz
  attr_accessor :foo
  alias :foo? :foo

  def example
  end
end

# YARD-documented attribute accessors with comments between them
class ExecutionResult
  # @return [Object, nil]
  attr_reader :value
  # @return [Exception, nil]
  attr_reader :handled_error
  # @return [Exception, nil]
  attr_reader :unhandled_error

  def example
  end
end

# attr_reader inside if/else branch — no offense (RuboCop skips if_type? parents)
if condition
  attr_reader :foo
else
  do_something
end

# attr_reader inside if/elsif branch
if condition
  attr_reader :foo
elsif other_condition
  do_something
end

# attr_writer inside case/when
case x
when :a
  attr_writer :foo
when :b
  do_something
end

# attr_accessor inside begin/rescue
begin
  attr_accessor :foo
rescue StandardError
  handle_error
end

# attr_reader inside begin/ensure
begin
  attr_reader :foo
ensure
  cleanup
end

# attr_accessor followed by else
if something
  attr_accessor :bar
else
  other_thing
end

# attr_accessor inside unless
unless condition
  attr_accessor :baz
else
  fallback
end
