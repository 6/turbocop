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
