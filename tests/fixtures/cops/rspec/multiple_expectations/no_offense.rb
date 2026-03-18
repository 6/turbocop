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

# RSpec.shared_examples with :aggregate_failures — nested examples inherit
RSpec.shared_examples 'importable', :aggregate_failures do
  it 'returns success' do
    expect(result).to include(status: 'success')
    expect(result[:records].count).to be(2)
  end

  it 'creates records' do
    expect { result }.to change(User, :count).by(2)
    expect(User.last).to have_attributes(name: 'test')
  end
end

# RSpec.shared_context with :aggregate_failures
RSpec.shared_context 'validated', :aggregate_failures do
  it 'validates fields' do
    expect(record).to be_valid
    expect(record.errors).to be_empty
  end
end

# pending/skip wrapping examples with single expectations — no offense on inner examples
pending 'deferred feature' do
  it 'has only one expectation' do
    expect(foo).to eq(bar)
  end
end

skip 'disabled feature' do
  it 'has only one expectation' do
    expect(foo).to eq(bar)
  end
end

# aggregate_failures deeply nested inside RSpec.shared_examples
RSpec.shared_examples 'nested groups', :aggregate_failures do
  describe '.process' do
    context 'when valid' do
      it 'succeeds' do
        expect(result).to eq(true)
        expect(errors).to be_empty
      end
    end
  end
end
