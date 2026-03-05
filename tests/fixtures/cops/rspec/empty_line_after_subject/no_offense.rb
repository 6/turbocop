RSpec.describe User do
  subject { described_class.new }

  let(:params) { foo }
end

RSpec.describe Post do
  subject! { described_class.new }

  let(:params) { foo }
end

RSpec.describe Comment do
  subject { described_class.new }
end

RSpec.describe Item do
  subject { described_class.new }
  # This is a comment

  it { is_expected.to be_valid }
end

RSpec.describe Order do
  subject { described_class.new }
  # First comment
  # Second comment

  it { is_expected.to be_valid }
end
