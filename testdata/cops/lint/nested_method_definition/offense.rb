def foo
  def bar
  ^^^^^^^ Lint/NestedMethodDefinition: Method definitions must not be nested. Use `lambda` instead.
    something
  end
end
def baz
  def qux
  ^^^^^^^ Lint/NestedMethodDefinition: Method definitions must not be nested. Use `lambda` instead.
    other
  end
end
def outer
  def inner
  ^^^^^^^^^ Lint/NestedMethodDefinition: Method definitions must not be nested. Use `lambda` instead.
    42
  end
end
