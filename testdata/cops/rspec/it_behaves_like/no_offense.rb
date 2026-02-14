it_behaves_like 'a foo'

describe Foo do
  it_behaves_like 'shared stuff'

  context 'nested' do
    it_behaves_like 'more stuff'
  end
end
