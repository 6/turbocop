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
