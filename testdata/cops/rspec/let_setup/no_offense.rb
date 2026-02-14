describe Foo do
  let!(:foo) { bar }

  before do
    foo
  end

  it 'does not use foo' do
    expect(baz).to eq(qux)
  end
end

describe Foo do
  let!(:foo) { bar }

  it 'uses foo' do
    foo
    expect(baz).to eq(qux)
  end
end
