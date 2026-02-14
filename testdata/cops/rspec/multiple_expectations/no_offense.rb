RSpec.describe Foo do
  it 'has a single expectation' do
    expect(foo).to eq(bar)
  end

  it 'also has one expectation' do
    expect(baz).to be_truthy
  end

  specify do
    is_expected.to be_valid
  end

  it { expect(true).to be(true) }
end
