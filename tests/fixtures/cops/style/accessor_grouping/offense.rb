class Foo
  attr_reader :bar1
  ^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
  attr_reader :bar2
  ^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
  attr_accessor :quux
  attr_reader :bar3, :bar4
  ^^^^^^^^^^^^^^^^^^^^^^^^ Style/AccessorGrouping: Group together all `attr_reader` attributes.
  other_macro :zoo
end
