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

# `private :method_name` makes the def private for ordering but does NOT act
# as a visibility block for subsequent defs. No offense when private method
# is last or only followed by more private methods.
class VisibilityDeclarationAtEnd
  CONST = 1
  def initialize
    @x = 1
  end
  def bar
    3
  end
  def foo
    2
  end
  private :foo
end

# `protected :method_name` similarly makes the def protected for ordering.
class ProtectedDeclarationAtEnd
  CONST = 1
  def initialize
    @x = 1
  end
  def qux
    3
  end
  def baz
    2
  end
  protected :baz
end

# Multiple symbol args: `private :foo, :bar` — no offense if private defs are last
class MultipleVisibilityArgsAtEnd
  include Comparable
  CONST = 1
  def initialize
    @x = 1
  end
  def gamma
    3
  end
  def alpha
    1
  end
  def beta
    2
  end
  private :alpha, :beta
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

# Inline private declaration where no public method follows: no offense
class InlinePrivateNoFollowUp
  include Comparable
  CONST = 1
  def initialize
    @x = 1
  end
  def validate_back_url(back_url)
    back_url
  end
  private :validate_back_url
end

# Inline protected followed by more protected: no offense
class InlineProtectedOrder
  def public_method
    1
  end
  def scm_entries
    2
  end
  protected :scm_entries
end
