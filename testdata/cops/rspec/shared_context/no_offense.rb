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

# shared_examples with let + it_behaves_like (example inclusions count as examples)
shared_examples 'literals that are frozen' do |o|
  let(:prefix) { o }

  it_behaves_like 'immutable objects', '[1, 2, 3]'
  it_behaves_like 'immutable objects', '%w(a b c)'
end

# shared_examples with include_examples
shared_examples 'mixed' do
  let(:x) { 1 }
  include_examples 'some examples'
end
