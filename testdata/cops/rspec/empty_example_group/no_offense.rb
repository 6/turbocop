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

# shared_context and shared_examples are not example groups
# and should not be flagged even without examples
shared_context 'with standard tweet info' do
  before { @link = 'https://example.com' }
  let(:full_name) { 'Test' }
end

shared_examples 'throttled endpoint' do
  let(:limit) { 25 }
  let(:period) { 5 }
end
