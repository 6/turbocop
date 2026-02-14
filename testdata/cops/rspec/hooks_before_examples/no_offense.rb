RSpec.describe User do
  before { setup }
  after { cleanup }

  it { is_expected.to be_valid }

  context 'nested' do
    before { more_setup }
    it { is_expected.to work }
  end

  include_examples 'shared stuff'
end
