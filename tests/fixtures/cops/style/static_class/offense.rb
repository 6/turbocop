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
