RSpec.describe MyClass do
  subject { "MyClass" }
end

module MyModule
  def some_method
  end
end

describe Foo do
  it 'works' do
  end
end

# Bare describe (no RSpec. prefix) inside a module is NOT flagged by RuboCop
module Decidim::Accountability
  describe ResultCell, type: :cell do
    it 'renders something' do
    end
  end
end

module MyNamespace
  describe SomeService do
    it 'does work' do
    end
  end
end
