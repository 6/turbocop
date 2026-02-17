class Foo
  private
  PRIVATE_CONST = 42
  ^^^^^^^^^^^^^^^^^^ Lint/UselessConstantScoping: Useless `private` access modifier for constant scope.
end

class Bar
  private
  MY_CONST = 'hello'
  ^^^^^^^^^^^^^^^^^^ Lint/UselessConstantScoping: Useless `private` access modifier for constant scope.
end

class Baz
  private
  X = 1
  ^^^^^ Lint/UselessConstantScoping: Useless `private` access modifier for constant scope.
end
