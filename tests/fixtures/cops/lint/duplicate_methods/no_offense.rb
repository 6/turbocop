class Foo
  def bar
    1
  end

  def baz
    2
  end
end

module MyMod
  def helper
    true
  end
end

# Instance and singleton methods with the same name are different
class Separate
  def foo
    :instance
  end

  def self.foo
    :class
  end
end

# Conditional method definitions should not be flagged
class Platform
  if RUBY_VERSION >= "3.0"
    def bar
      :modern
    end
  else
    def bar
      :legacy
    end
  end
end

# alias to self is allowed
class WithAlias
  alias foo foo
  def foo
    1
  end
end
