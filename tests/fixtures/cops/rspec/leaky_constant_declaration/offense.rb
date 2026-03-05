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

RSpec.shared_examples 'shared example' do
  CONSTANT = "Accessible as ::CONSTANT".freeze
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyConstantDeclaration: Stub constant instead of declaring explicitly.
end

describe SomeClass do
  specify do
    CONSTANT = "Accessible as ::CONSTANT".freeze
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyConstantDeclaration: Stub constant instead of declaring explicitly.
  end
end

# Constants nested inside control structures should still be flagged
describe SomeClass do
  if some_condition
    NESTED_CONST = "leaky"
    ^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyConstantDeclaration: Stub constant instead of declaring explicitly.
  end
end

describe SomeClass do
  unless some_condition
    class NestedClass
    ^^^^^^^^^^^^^^^^^ RSpec/LeakyConstantDeclaration: Stub class constant instead of declaring explicitly.
    end
  end
end

describe SomeClass do
  case something
  when :foo
    module NestedModule
    ^^^^^^^^^^^^^^^^^^^ RSpec/LeakyConstantDeclaration: Stub module constant instead of declaring explicitly.
    end
  end
end

describe SomeClass do
  begin
    RESCUE_CONST = "leaky"
    ^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeakyConstantDeclaration: Stub constant instead of declaring explicitly.
  rescue StandardError
    nil
  end
end
