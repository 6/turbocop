describe SomeClass do
  CONSTANT = "Accessible as ::CONSTANT".freeze
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyConstantDeclaration: Stub constant instead of declaring explicitly.
end

describe SomeClass do
  class DummyClass < described_class
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyConstantDeclaration: Stub class constant instead of declaring explicitly.
  end
end

describe SomeClass do
  module DummyModule
  ^^^^^^^^^^^^^^^^^^ RSpec/LeakyConstantDeclaration: Stub module constant instead of declaring explicitly.
  end
end
