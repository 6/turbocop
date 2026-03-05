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

# Hooks with different metadata should not be flagged
describe MetadataExample do
  before(:each, :unix_only) do
    setup_unix
  end

  before(:each) do
    setup_normal
  end

  it { expect(true).to eq(true) }
end

# Hooks with different metadata (symbol vs none)
describe MetadataExample2 do
  before(:example) { foo }
  before(:example, :special_case) { bar }

  it { expect(true).to eq(true) }
end

# after hooks with different scopes (explicit :each vs :all)
describe AfterScopeExample do
  after do
    cleanup_general
  end

  after(:all) do
    cleanup_once
  end

  it { expect(true).to eq(true) }
end

# Hooks with different keyword metadata values
describe KeywordMetadata do
  before(:example, special_case: true) { bar }
  before(:example, special_case: false) { baz }

  it { expect(true).to eq(true) }
end

# Shared context should NOT trigger (not an example_group?)
shared_context 'common setup' do
  before { setup_shared }
  before { setup_more }
end

# Shared examples should NOT trigger
shared_examples 'common behavior' do
  before { setup_shared }
  before { setup_more }
end

# Shared examples_for should NOT trigger
shared_examples_for 'common behavior' do
  before { setup_shared }
  before { setup_more }
end

# Different hook types are separate groups (before vs prepend_before)
describe DifferentHookTypes do
  before { setup_one }
  prepend_before { setup_two }
  append_before { setup_three }

  it { expect(true).to eq(true) }
end

# Hooks inside class methods should not be flagged
describe ClassMethodHooks do
  before { main_setup }

  def self.setup
    before { class_method_setup }
  end

  it { expect(true).to eq(true) }
end

# Hooks in nested example groups are separate scopes
describe NestedScopes do
  before { outer_setup }

  context 'inner' do
    before { inner_setup }
    it { expect(true).to eq(true) }
  end
end

# around hooks are never flagged
describe AroundHooks do
  around { |example| example.run }
  around { |example| example.run }

  it { expect(true).to eq(true) }
end
