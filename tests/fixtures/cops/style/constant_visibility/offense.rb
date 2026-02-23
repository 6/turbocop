class Foo
  BAR = 42
  ^^^^^^^^ Style/ConstantVisibility: Explicitly make `BAR` public or private using either `#public_constant` or `#private_constant`.
end

module Baz
  QUX = 'hello'
  ^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `QUX` public or private using either `#public_constant` or `#private_constant`.
end

class Quux
  include Bar
  BAZ = 42
  ^^^^^^^^ Style/ConstantVisibility: Explicitly make `BAZ` public or private using either `#public_constant` or `#private_constant`.
  private_constant :FOO
end
