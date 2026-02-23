class Foo
  attr :something, true
  ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
end

class Bar
  attr :one, :two, :three
  ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
end

class Baz
  attr :name
  ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
end
