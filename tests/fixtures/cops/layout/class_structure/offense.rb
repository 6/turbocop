class Foo
  def bar
    1
  end
  include Comparable
  ^^^^^^^ Layout/ClassStructure: ModuleInclusion is expected to appear before PublicMethods.
end

class Baz
  def initialize
    @x = 1
  end
  CONST = 1
  ^^^^^ Layout/ClassStructure: Constants is expected to appear before Initializer.
end

class Qux
  def qux_method
    2
  end
  include Enumerable
  ^^^^^^^ Layout/ClassStructure: ModuleInclusion is expected to appear before PublicMethods.
end
