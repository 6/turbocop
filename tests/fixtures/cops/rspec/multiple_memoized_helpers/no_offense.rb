describe Foo do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }
  let(:d) { 4 }
  let(:e) { 5 }

  it { expect(a + b + c + d + e).to eq(15) }
end

describe Bar do
  let(:x) { 'x' }

  it { expect(x).to eq('x') }
end

# Nested context: 3 parent + 2 nested = 5, exactly at the Max
describe Baz do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }

  context 'nested stays under limit' do
    let(:d) { 4 }
    let(:e) { 5 }

    it { expect(true).to be true }
  end
end

# Overriding lets in child context do not increase count
describe Overrides do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }
  let(:d) { 4 }
  let(:e) { 5 }

  context 'redefines some parent lets' do
    let(:a) { 10 }
    let(:b) { 20 }

    it { expect(a).to eq(10) }
  end
end

# Helpers nested in if/case/begin still count but stay within limit
describe NestedButUnderLimit do
  let(:a) { 1 }
  let(:b) { 2 }

  if ENV['CI']
    let(:c) { 3 }
    let(:d) { 4 }
    let(:e) { 5 }
  end
end

# Helpers inside it_behaves_like blocks are in a different scope (not counted)
describe IncludeScope do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }
  let(:d) { 4 }
  let(:e) { 5 }

  it_behaves_like 'a widget' do
    let(:f) { 6 }
    let(:g) { 7 }
  end
end

# Helpers inside include_examples blocks are in a different scope
describe IncludeExamplesScope do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }
  let(:d) { 4 }
  let(:e) { 5 }

  include_examples 'shared stuff' do
    let(:f) { 6 }
  end
end

# Helpers inside include_context blocks are in a different scope
describe IncludeContextScope do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }
  let(:d) { 4 }
  let(:e) { 5 }

  include_context 'common setup' do
    let(:f) { 6 }
  end
end

# Helpers inside it_should_behave_like blocks are in a different scope
describe ItShouldBehaveLikeScope do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }
  let(:d) { 4 }
  let(:e) { 5 }

  it_should_behave_like 'a thing' do
    let(:f) { 6 }
  end
end

# Shared examples under the limit are fine
shared_examples 'under limit' do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }

  it { expect(a).to eq(1) }
end

# Shared context under the limit
shared_context 'under limit context' do
  let(:a) { 1 }
  let(:b) { 2 }
end

# Block-pass form: let(:foo, &bar) under limit is fine
describe BlockPassUnderLimit do
  let(:a, &method(:something_a))
  let(:b, &method(:something_b))

  it { expect(a).to eq(1) }
end

# Bare let(:foo) without a block should NOT be counted
describe BareLetNotCounted do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }
  let(:d) { 4 }
  let(:e) { 5 }
  let(:f)

  it { expect(a).to eq(1) }
end

# Helpers inside RSpec.shared_context (with receiver) should NOT leak to parent.
# Parent has 3 lets; inner RSpec.shared_context adds 2 own but they don't leak.
# The inner RSpec.shared_context inherits 3 + 2 = 5 (at limit, no offense).
describe NoLeakFromRSpecSharedContext do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }

  RSpec.shared_context 'inner setup' do
    let(:d) { 4 }
    let(:e) { 5 }
  end
end

# RSpec.shared_context under the limit
RSpec.shared_context 'under limit with receiver' do
  let(:a) { 1 }
  let(:b) { 2 }
end

# let (:name) with space before parens - under the limit
describe SpacedLetUnderLimit do
  let (:a) { 1 }
  let (:b) { 2 }
  let (:c) { 3 }
  let (:d) { 4 }
  let (:e) { 5 }
end
