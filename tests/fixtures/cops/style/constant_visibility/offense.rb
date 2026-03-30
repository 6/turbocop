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

module Test
  IndexMapping::Interpolation = ::Google::Protobuf::DescriptorPool.generated_pool.lookup("test").enummodule
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `Interpolation` public or private using either `#public_constant` or `#private_constant`.
end

class InstallGenerator
  ::InvalidChannel = InvalidChannel
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `InvalidChannel` public or private using either `#public_constant` or `#private_constant`.
  ::ConflictingOptions = ConflictingOptions
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `ConflictingOptions` public or private using either `#public_constant` or `#private_constant`.
end

module Skyline
  class Engine
    Skyline::Engine::SESSION_OPTIONS = {}
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `SESSION_OPTIONS` public or private using either `#public_constant` or `#private_constant`.
  end
end

module Proto
  Trace::CachePolicy = lookup("Trace.CachePolicy").msgclass
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `CachePolicy` public or private using either `#public_constant` or `#private_constant`.
  Trace::CachePolicy::Scope = lookup("Trace.CachePolicy.Scope").enummodule
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `Scope` public or private using either `#public_constant` or `#private_constant`.
end
