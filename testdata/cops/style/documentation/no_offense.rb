# A documented class
class Foo
  def method
  end
end

# A documented module
module Bar
  def method
  end
end

# Class with methods
class MyClass
  def method
  end
end

# Module with methods
module MyModule
  def method
  end
end

# Multiline docs
# with extra info
class Documented
  def method
  end
end

# Empty class (no docs needed)
class EmptyClass
end

# Namespace module (only classes inside)
module TestNamespace
  class A; end
  class B; end
end

# Namespace class with constants
class TestConstants
  A = Class.new
  B = 1
end

# :nodoc: suppresses docs
class Undocumented #:nodoc:
  def method
  end
end

# Include-only module
module Mixin
  include Comparable
end

# Extend-only module
module ExtendMixin
  extend ActiveSupport
end

# Module with private_constant
module WithPrivate
  class Internal
  end
  private_constant :Internal
end
