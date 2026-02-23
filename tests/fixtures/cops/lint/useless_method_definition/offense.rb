class Foo
  def bar
  ^^^ Lint/UselessMethodDefinition: Useless method definition detected. The method just delegates to `super`.
    super
  end

  def baz(x, y)
  ^^^ Lint/UselessMethodDefinition: Useless method definition detected. The method just delegates to `super`.
    super(x, y)
  end

  def qux
  ^^^ Lint/UselessMethodDefinition: Useless method definition detected. The method just delegates to `super`.
    super()
  end
end
