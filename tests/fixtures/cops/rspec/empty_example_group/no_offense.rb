describe Foo do
  context 'when bar' do
    it { expect(true).to be(true) }
  end

  describe '#baz' do
    specify { expect(subject.baz).to eq(1) }
  end

  context 'with includes' do
    include_examples 'shared stuff'
  end

  it 'not implemented'
end

# shared_context and shared_examples are not example groups
# and should not be flagged even without examples
shared_context 'with standard tweet info' do
  before { @link = 'https://example.com' }
  let(:full_name) { 'Test' }
end

shared_examples 'throttled endpoint' do
  let(:limit) { 25 }
  let(:period) { 5 }
end

# `its` is a valid example method (rspec-its gem)
describe Record do
  its(:name) { is_expected.to eq('test') }
end

# `pending` without block counts as example
describe Validator do
  pending 'too hard to specify'
end

# examples inside iterators count
describe 'monthly report' do
  [1, 2, 3].each do |page|
    it { expect(page).to be > 0 }
  end
end

# examples inside custom blocks count
context 'with role' do
  with_permissions :admin do
    it { expect(subject).to be_allowed }
  end
end

# it_should_behave_like counts as content
describe Integration do
  context 'when complete' do
    it_should_behave_like 'a valid record'
  end
end
