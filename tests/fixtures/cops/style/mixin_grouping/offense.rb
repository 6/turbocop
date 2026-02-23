class Foo
  include Bar, Qux
  ^^^^^^^^^^^^^^^^ Style/MixinGrouping: Put `include` mixins in separate statements.
end

class Baz
  extend A, B
  ^^^^^^^^^^^ Style/MixinGrouping: Put `extend` mixins in separate statements.
end

class Quux
  prepend X, Y, Z
  ^^^^^^^^^^^^^^^^ Style/MixinGrouping: Put `prepend` mixins in separate statements.
end
