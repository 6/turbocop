def foo
  def bar
  ^^^^^^^ Lint/NestedMethodDefinition: Method definitions must not be nested. Use `lambda` instead.
    something
  end
end