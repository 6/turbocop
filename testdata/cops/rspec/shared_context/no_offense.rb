shared_context 'foo' do
  let(:foo) { :bar }

  it 'performs actions' do
  end
end

shared_examples 'bar' do
  subject(:foo) { 'foo' }
  let(:bar) { :baz }
  before { initialize }

  it 'works' do
  end
end

shared_context 'empty' do
end
