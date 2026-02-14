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
