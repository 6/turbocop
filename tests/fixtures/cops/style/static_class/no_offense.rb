module Foo
  def self.bar
    42
  end
end

class Bar
  def instance_method
    'hello'
  end
end

class Baz
  CONST = 1
  def self.foo
    CONST
  end
end

class Child < Parent
  def self.class_method
    42
  end
end

class WithInclude
  include SomeModule
  def self.class_method
    42
  end
end

class WithPrepend
  prepend SomeModule
  def self.class_method
    42
  end
end
