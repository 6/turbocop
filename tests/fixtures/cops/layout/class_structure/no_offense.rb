class Foo
  include Comparable
  CONST = 1
  def initialize
    @x = 1
  end
  def bar
    2
  end
end

# Class method before initialize (def self.foo is public_class_methods)
class Bar
  def self.create
    new
  end
  def initialize
    @x = 1
  end
  def bar
    2
  end
end

# Private constant (followed by private_constant) should be ignored for ordering
class Baz
  private

  INTERNAL = 42
  private_constant :INTERNAL

  def compute
    INTERNAL
  end
end

# Macros like attr_reader should be ignored (not in ExpectedOrder)
class Qux
  attr_reader :name
  def initialize(name)
    @name = name
  end
  def greet
    "Hi"
  end
end
