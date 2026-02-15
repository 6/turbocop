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

# shared_examples are NOT example groups for this cop's purposes
# so hooks after shared_examples are allowed
RSpec.describe Widget do
  shared_examples 'common behavior' do
    it 'works' do
      expect(true).to be true
    end
  end

  before { setup_widget }

  it_behaves_like 'common behavior'
end
