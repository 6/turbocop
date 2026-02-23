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
