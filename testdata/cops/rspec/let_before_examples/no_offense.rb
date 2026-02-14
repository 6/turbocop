RSpec.describe User do
  let(:foo) { bar }

  it { is_expected.to be_valid }

  context 'nested' do
    let(:baz) { qux }
    it { is_expected.to work }
  end

  include_examples 'shared stuff'
end
