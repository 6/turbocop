describe Foo do
  before { bar }
  after { baz }
  around { |t| t.run }

  it { expect(true).to be(true) }
end

describe Bar do
  before { setup }

  describe '.baz' do
    before { more_setup }
    it { expect(1).to eq(1) }
  end
end
