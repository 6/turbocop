describe MyClass do
  context 'when foo' do
    context 'when bar' do
      it { expect(true).to be(true) }
    end
  end

  context 'when qux' do
    it { expect(1).to eq(1) }
  end
end

RSpec.describe AnotherClass do
  it 'works' do
    expect(subject).to be_valid
  end
end

# shared_examples inside describe should NOT increment nesting
describe SomeClass do
  shared_examples 'valid record' do
    context 'when active' do
      context 'when verified' do
        it { expect(subject).to be_valid }
      end
    end
  end

  context 'first' do
    context 'second' do
      it { expect(true).to eq(true) }
    end
  end
end

# shared_context inside describe should NOT increment nesting
describe AnotherExample do
  shared_context 'with setup' do
    context 'when configured' do
      context 'when ready' do
        it { expect(subject).to be_ready }
      end
    end
  end
end

# Top-level shared_examples_for does not count toward nesting
# (3 nested contexts inside = nesting 3, not exceeding Max of 3)
shared_examples_for 'reusable behavior' do
  context 'level one' do
    context 'level two' do
      context 'level three' do
        it { expect(subject).to be_valid }
      end
    end
  end
end

# Top-level shared_examples does not count toward nesting
shared_examples 'another pattern' do
  describe 'some feature' do
    context 'first case' do
      context 'second case' do
        it { expect(true).to eq(true) }
      end
    end
  end
end

# Top-level shared_context does not count toward nesting
shared_context 'with dependencies' do
  context 'when enabled' do
    context 'when configured' do
      context 'when ready' do
        it { expect(subject).to be_ready }
      end
    end
  end
end

# Module with sibling top-level statements is NOT unwrapped
# (RuboCop's TopLevelGroup only unwraps sole top-level module/class)
module MyNamespace
  describe SomeService do
    context 'when active' do
      context 'when verified' do
        context 'when ready' do
          it { expect(subject).to be_ready }
        end
      end
    end
  end
end
