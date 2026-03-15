class Foo
  def initialize
    return
  end
end

class Bar
  def initialize(x)
    @x = x
  end
end

class Baz
  def some_method
    return 42
  end
end

# return inside lambda within initialize (returns from lambda, not initialize)
class WithLambda
  def initialize
    @handler = lambda do
      return :early if condition?
      process
    end
  end
end

# return inside define_method within initialize
class WithDefineMethod
  def initialize
    define_method(:foo) do
      return bar
    end
  end
end

# return inside define_singleton_method within initialize
class WithDefineSingletonMethod
  def initialize
    define_singleton_method(:foo) do
      return bar
    end
  end
end

# return without value in setter method
class WithSetter
  def foo=(bar)
    return
  end
end

# class method called initialize (not an instance initializer)
class WithClassInit
  def self.initialize
    return :qux if bar?
  end
end

# return inside nested def within initialize
class WithNestedDef
  def initialize
    def foo
      return bar
    end
  end
end
