describe Foo do
  subject(:foo) { described_class.new }

  before do
    allow(other_obj).to receive(:bar).and_return(baz)
  end

  it 'does something' do
    expect(foo.bar).to eq(baz)
  end
end

describe Bar do
  let(:bar) { double }

  before do
    allow(bar).to receive(:baz)
  end
end
