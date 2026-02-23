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

# before :all and before :each (default) are different scope types
describe Qux do
  before :all do
    setup_once
  end

  before do
    setup_each
  end

  it { expect(true).to eq(true) }
end
