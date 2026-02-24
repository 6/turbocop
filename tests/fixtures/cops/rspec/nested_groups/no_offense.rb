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
