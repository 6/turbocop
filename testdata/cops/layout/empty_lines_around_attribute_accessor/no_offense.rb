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
