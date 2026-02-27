class Foo
  attr :writable, true
  ^^^^ Style/Attr: Do not use `attr`. Use `attr_accessor` instead.
end

class Bar
  attr :one, :two, :three
  ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
end

class Baz
  attr :name
  ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
end

class Qux
  attr :readable, false
  ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
end
