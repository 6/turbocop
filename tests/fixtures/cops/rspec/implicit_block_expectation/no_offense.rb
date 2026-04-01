# Non-lambda subject — is_expected should NOT be flagged
describe 'value subject' do
  subject { User.new }
  it { is_expected.to be_valid }
end

# Explicit block expectations are fine
expect { boom }.to raise_error(StandardError)
expect { action }.to change { something }.to(new_value)

# Non-lambda subject with eq
describe 'plain' do
  subject { 'normal' }
  it { is_expected.to eq(something) }
end

# No subject defined — don't flag
shared_examples 'subject is defined somewhere else' do
  it { is_expected.to change { something }.to(new_value) }
end

# Child context overrides lambda subject with non-lambda
describe 'outer' do
  subject { -> { boom } }
  context 'inner' do
    subject { 'normal' }
    it { is_expected.to eq(something) }
  end
end

# Lambda nested inside hash — not a direct lambda subject
describe 'hash with lambda' do
  subject { {hash: -> { boom }} }
  it { is_expected.to be(something) }
end

# Explicit expect subject
describe 'explicit' do
  subject { -> { boom } }
  it { expect(subject).to eq(42) }
end

# Standalone is_expected with block matcher but no subject in scope
is_expected.to change { something }.to(new_value)
is_expected.to raise_error(StandardError)
is_expected.to throw_symbol(:halt)

# RSpec.describe with non-lambda subject — should not flag
RSpec.describe 'receiver non-lambda' do
  subject { User.new }
  it { is_expected.to be_valid }
end

# Lambda subject inside shared_examples — RuboCop's multi_statement_example_group?
# excludes shared groups, so nearest_subject skips them and doesn't find the subject.
RSpec.describe 'shared examples with lambda subject' do
  shared_examples 'common behavior' do
    subject do
      -> { described_class.new.call(account) }
    end
    it { is_expected.to change { something } }
    it { should change { something } }
  end
end
