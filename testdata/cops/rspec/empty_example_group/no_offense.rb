describe Foo do
  context 'when bar' do
    it { expect(true).to be(true) }
  end

  describe '#baz' do
    specify { expect(subject.baz).to eq(1) }
  end

  context 'with includes' do
    include_examples 'shared stuff'
  end

  it 'not implemented'
end
