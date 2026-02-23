RSpec.describe User do
  subject { described_class.new }

  let(:params) { foo }

  context 'nested' do
    subject { described_class.new }
    it { is_expected.to be_valid }
  end
end

RSpec.describe Post do
  subject { described_class.new }

  before { setup }
  it { is_expected.to be_present }
end
