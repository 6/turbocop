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

# include_context is NOT an example include, so hooks after it are allowed
RSpec.describe Config do
  include_context 'mock console output'

  before do
    setup_config
  end

  after do
    cleanup_config
  end

  it 'validates configuration' do
    expect(config).to be_valid
  end
end

# Multiple include_context calls followed by hooks are fine
RSpec.describe Loader do
  include_context 'cli spec behavior'
  include_context 'mock console output'

  before { initialize_loader }
  after { reset_loader }

  it 'loads files' do
    expect(loader.files).not_to be_empty
  end
end
