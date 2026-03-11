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

# Multiple class methods in correct order
class MultipleClassMethods
  def self.first_class_method
    1
  end
  def self.second_class_method
    2
  end
  def initialize
    @x = 1
  end
  def instance_method
    3
  end
end

# Protected then private in correct order
class VisibilityOrder
  def public_method
    1
  end
  protected
  def protected_method
    2
  end
  private
  def private_method
    3
  end
end

# `private :method_name` is a visibility declaration, NOT a section marker;
# it should be ignored (not in ExpectedOrder), not classified as private_methods.
class VisibilityDeclaration
  CONST = 1
  def initialize
    @x = 1
  end
  def foo
    2
  end
  private :foo
  def bar
    3
  end
end

# `protected :method_name` should also be ignored
class ProtectedDeclaration
  CONST = 1
  def initialize
    @x = 1
  end
  def baz
    2
  end
  protected :baz
  def qux
    3
  end
end

# Multiple symbol args: `private :foo, :bar`
class MultipleVisibilityArgs
  include Comparable
  CONST = 1
  def initialize
    @x = 1
  end
  def alpha
    1
  end
  def beta
    2
  end
  private :alpha, :beta
  def gamma
    3
  end
end

# `private def foo` IS a def modifier and should be classified as private_methods
# but should not affect subsequent methods' classification
class DefModifierDoesNotAffectNext
  include Comparable
  CONST = 1
  def initialize
    @x = 1
  end
  def bar
    2
  end
end

# Singleton class (class << self) in correct order
class << self
  CONST = 1
  def some_method
    2
  end
end
