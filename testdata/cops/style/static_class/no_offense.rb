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
