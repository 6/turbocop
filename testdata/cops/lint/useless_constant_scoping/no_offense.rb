class Foo
  PRIVATE_CONST = 42
  private_constant :PRIVATE_CONST
end

class Bar
  PUBLIC_CONST = 42
end

class Baz
  private
  def my_method; end
end
