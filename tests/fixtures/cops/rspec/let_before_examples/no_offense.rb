RSpec.describe User do
  let(:foo) { bar }

  it { is_expected.to be_valid }

  context 'nested' do
    let(:baz) { qux }
    it { is_expected.to work }
  end

  include_examples 'shared stuff'
end

# shared_examples blocks should NOT count as "examples seen"
# so let after shared_examples is allowed
RSpec.describe Another do
  shared_examples 'throttled endpoint' do
    it 'works' do
      expect(true).to be true
    end
  end

  let(:remote_ip) { '1.2.3.5' }
  let(:discriminator) { remote_ip }

  describe 'throttle' do
    let(:throttle) { 'test' }
    it_behaves_like 'throttled endpoint'
  end
end

# Inside shared_examples, let after examples is allowed
shared_examples 'detect/correct empty case, accept non-empty case' do
  it 'registers an offense' do
    expect_offense(source)
  end

  let(:source_with_case) { source.sub('case', 'case :a') }

  it 'accepts the source with case' do
    expect_no_offenses(source_with_case)
  end
end
