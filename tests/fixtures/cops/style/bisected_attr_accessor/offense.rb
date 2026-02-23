class Foo
  attr_reader :bar
              ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :bar`.
  attr_writer :bar
              ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :bar`.
  other_macro :something
end

class Baz
  attr_reader :qux
              ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :qux`.
  attr_writer :qux
              ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :qux`.
end
