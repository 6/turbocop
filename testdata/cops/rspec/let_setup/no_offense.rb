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

# let! name referenced in a sibling let! body — should not be flagged
describe Widget do
  let!(:user) { create(:user) }
  let!(:post) { create(:post, author: user) }

  it 'creates a post' do
    expect(post).to be_valid
  end
end
