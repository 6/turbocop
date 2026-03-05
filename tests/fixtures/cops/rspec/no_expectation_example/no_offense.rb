RSpec.describe Foo do
  it { expect(baz).to be_truthy }

  it { should be_valid }

  it { should_not be_empty }

  it 'has an expectation' do
    expect(subject.name).to eq('foo')
  end

  it 'uses is_expected' do
    is_expected.to be_present
  end

  it 'uses are_expected' do
    are_expected.to all(be_positive)
  end

  it 'uses should_receive' do
    should_receive(:foo)
  end

  it 'uses should_not_receive' do
    should_not_receive(:bar)
  end

  it 'not implemented'

  # x-prefixed examples are excluded (RuboCop SkipOrPending mixin)
  xit { bar }
  xspecify { baz }
  xexample { qux }
  xscenario { quux }

  # Skip/pending metadata excludes from check
  it 'skipped', :skip do
    bar
  end
  it 'pending', :pending do
    bar
  end
  it 'skip with reason', skip: 'not ready' do
    bar
  end
  it 'pending with reason', pending: 'WIP' do
    bar
  end

  # In-body pending/skip counts as expectation
  it 'has pending' do
    pending
    bar
  end
  it 'has skip' do
    skip
    bar
  end
end
