class Foo
  def initialize
    return 42
    ^^^^^^^^^ Lint/ReturnInVoidContext: Do not return a value in `initialize`.
  end
end

class Bar
  def initialize(x)
    return x if x.nil?
    ^^^^^^^^ Lint/ReturnInVoidContext: Do not return a value in `initialize`.
  end
end

class Baz
  def initialize
    return 'hello'
    ^^^^^^^^^^^^^^ Lint/ReturnInVoidContext: Do not return a value in `initialize`.
  end
end

class WithSetter
  def foo=(bar)
    return 42
    ^^^^^^^^^ Lint/ReturnInVoidContext: Do not return a value in `foo=`.
  end
end

# return with value inside a regular block within initialize (still an offense)
class WithBlock
  def initialize
    items.each do
      return :qux
      ^^^^^^^^^^^ Lint/ReturnInVoidContext: Do not return a value in `initialize`.
    end
  end
end

# return with value inside proc within initialize (still an offense - proc doesn't change scope)
class WithProc
  def initialize
    proc do
      return :qux
      ^^^^^^^^^^^ Lint/ReturnInVoidContext: Do not return a value in `initialize`.
    end
  end
end

class WithIndexWriter
  def []=(key, value)
    return value
    ^^^^^^^^^^^^ Lint/ReturnInVoidContext: Do not return a value in `[]=`.
  end
end

class WithClassSetter
  def self.mode=(value)
    return value
    ^^^^^^^^^^^^ Lint/ReturnInVoidContext: Do not return a value in `mode=`.
  end
end

class WithClassIndexWriter
  def self.[]=(key, value)
    return value
    ^^^^^^^^^^^^ Lint/ReturnInVoidContext: Do not return a value in `[]=`.
  end
end
