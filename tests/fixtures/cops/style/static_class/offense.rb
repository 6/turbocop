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
