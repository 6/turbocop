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

  # Single should-style expectations are fine
  it 'uses should once' do
    should eq(1)
  end

  it 'uses should_not once' do
    should_not eq(1)
  end

  it 'uses are_expected once' do
    are_expected.to include(1)
  end

  it 'uses should_receive once' do
    should_receive(:foo)
  end

  it 'uses should_not_receive once' do
    should_not_receive(:foo)
  end

  # aggregate_failures metadata on example — skip
  it 'many expectations with aggregate_failures', :aggregate_failures do
    expect(foo).to eq(bar)
    expect(baz).to eq(bar)
  end

  # aggregate_failures: true keyword — skip
  it 'keyword aggregate_failures', aggregate_failures: true do
    expect(foo).to eq(bar)
    expect(baz).to eq(bar)
  end

  # aggregate_failures block counts as single expectation
  it 'aggregates failures in a block' do
    aggregate_failures do
      expect(foo).to eq(bar)
      expect(baz).to eq(bar)
    end
  end
end

# aggregate_failures on example group — all nested examples skip
describe Foo, :aggregate_failures do
  it 'inherits aggregate_failures' do
    expect(foo).to eq(bar)
    expect(baz).to eq(bar)
  end
end
