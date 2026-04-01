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

# Stabby lambda subject — RuboCop does not flag this.
# RuboCop's lambda? pattern only matches proc/lambda/Proc.new call forms,
# not the -> {} syntax (which is a distinct lambda AST node, not a block
# wrapping a send to :lambda).
describe 'stabby lambda subject' do
  subject { -> { do_something } }
  it { is_expected.to change { something } }
end

# Stabby lambda with subject!
describe 'stabby lambda eager' do
  subject! { -> { boom } }
  it { is_expected.to terminate }
end

# Stabby lambda with named subject
describe 'stabby lambda named' do
  subject(:action) { -> { boom } }
  it { is_expected.to change { something }.to(new_value) }
end

# Stabby lambda inherited from outer group
describe 'stabby outer' do
  subject { -> { boom } }
  context 'inner' do
    it { is_expected.to change { something }.to(new_value) }
  end
end

# Stabby lambda with should/should_not
describe 'stabby should variants' do
  subject { -> { boom } }
  it { should change { something }.to(new_value) }
  it { should_not change { something }.to(new_value) }
end

# Stabby lambda with custom method
describe 'stabby custom method' do
  subject { -> { process(input) } }
  its_call('value') { is_expected.to ret([result]) }
end

# Stabby lambda with RSpec.describe
RSpec.describe 'stabby receiver' do
  subject { ->(source) { process(source) } }
  its_call('value') { is_expected.to ret([result]) }
end

# Stabby lambda with multiple contexts
describe 'stabby multiple' do
  subject { -> { described_class.run(args) } }
  context 'missing file' do
    let(:file) { 'missing.rb' }
    it { is_expected.to terminate.with_code(1) }
  end
  context 'unchanged file' do
    let(:file) { 'spec/fixtures/valid' }
    it { is_expected.to terminate }
  end
end
