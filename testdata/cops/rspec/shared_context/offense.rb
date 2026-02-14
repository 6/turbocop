shared_context 'foo' do
^^^^^^^^^^^^^^^^^^^^ RSpec/SharedContext: Use `shared_examples` when you don't define context.
  it 'performs actions' do
  end
end

shared_examples 'bar' do
^^^^^^^^^^^^^^^^^^^^^ RSpec/SharedContext: Use `shared_context` when you don't define examples.
  let(:foo) { :bar }
end

shared_examples 'baz' do
^^^^^^^^^^^^^^^^^^^^^ RSpec/SharedContext: Use `shared_context` when you don't define examples.
  before do
    foo
  end
end
