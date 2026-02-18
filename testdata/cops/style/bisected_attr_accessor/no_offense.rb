class Foo
  attr_accessor :bar
  attr_reader :baz
  attr_writer :qux
  other_macro :something
end

class Bar
  attr_reader :x
end
