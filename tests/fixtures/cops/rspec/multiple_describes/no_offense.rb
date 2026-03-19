describe MyClass do
  it 'works' do
    expect(true).to eq(true)
  end
end

shared_examples_for 'behaves' do
end

shared_examples_for 'misbehaves' do
end

# Block-argument style (&proc) should not be counted as a top-level example group.
# RuboCop's on_block only fires for BlockNode, not BlockArgumentNode (&proc).
describe 'Conditional feature', if: condition, &(proc do
  it 'works' do end
end)

# Single describe inside a module wrapper should not trigger
module Pronto
  module Formatter
    describe '.register' do
    end
  end
end

# Single describe inside a class wrapper should not trigger
class Treat::Specs::Workers::Agnostic
  describe 'Something' do
  end
end

# Module with require before it — multiple top-level statements, no unwrapping
require 'spec_helper'
module Foo
  describe 'bar' do
  end
  describe 'baz' do
  end
end
