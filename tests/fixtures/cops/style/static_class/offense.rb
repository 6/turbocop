class Foo
^^^^^^^^^ Style/StaticClass: Prefer modules to classes with only class methods.
  def self.bar
    42
  end
end

class Bar
^^^^^^^^^ Style/StaticClass: Prefer modules to classes with only class methods.
  def self.baz
    'hello'
  end
  def self.qux
    'world'
  end
end

class Utils
^^^^^^^^^^^ Style/StaticClass: Prefer modules to classes with only class methods.
  def self.helper
    true
  end
end

class WithConstant
^^^^^^^^^^^^^^^^^^ Style/StaticClass: Prefer modules to classes with only class methods.
  CONST = 1
  def self.foo
    CONST
  end
end

class WithExtend
^^^^^^^^^^^^^^^^ Style/StaticClass: Prefer modules to classes with only class methods.
  extend SomeModule
  def self.class_method; end
end

class WithSclass
^^^^^^^^^^^^^^^^ Style/StaticClass: Prefer modules to classes with only class methods.
  def self.class_method; end

  class << self
    def other_class_method; end
  end
end

class WithSclassAssignment
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/StaticClass: Prefer modules to classes with only class methods.
  class << self
    SETTING = 1
    def configure; end
  end
end

class WithEmptySclass
^^^^^^^^^^^^^^^^^^^^^ Style/StaticClass: Prefer modules to classes with only class methods.
  class << self
  end
end

class WithMultiWrite
^^^^^^^^^^^^^^^^^^^^ Style/StaticClass: Prefer modules to classes with only class methods.
  VERSION = "4.2.8"
  MAJOR, MINOR, TINY = VERSION.split(".")
end

class WithOnlyMultiWrite
^^^^^^^^^^^^^^^^^^^^^^^^ Style/StaticClass: Prefer modules to classes with only class methods.
  MAJOR, MINOR, TINY = "1.2.3".split(".")
end

module Wrapper
  class NestedStatic
  ^^^^^^^^^^^^^^^^^^ Style/StaticClass: Prefer modules to classes with only class methods.
    @@edited = nil
    @@switch_index = 0
    @@dash_prefix, @@dash_suffix = nil, nil
  end
end
