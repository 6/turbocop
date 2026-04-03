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

# :nodoc: all on outer module suppresses inner classes
module Outer #:nodoc: all
  class Inner
    def method
    end
  end

  module Nested
    def method
    end
  end
end

# :nodoc: all on class suppresses inner classes
class Base # :nodoc: all
  class Helper
    def method
    end
  end
end

unless defined?(ScopedDocumented)
  # Real doc
  class ScopedDocumented
    def method
    end
  end
end

class A::B::Test #:nodoc:
  def method
  end
end

# Class expression assigned inside a block is not top-level documentation
describe Foo do
  before do
    # Namespace docs
    module Testing; end
    @memory_class = class Testing::MyMemory < Parent
      self
    end
  end
end

# FP fix: class with class << self containing only include/prepend
class Cache
  class << self
    prepend Mixin
  end
end

# FP fix: module with class << self containing only include
module Marshal
  class << self
    include MarshalAutoloader
  end
end
