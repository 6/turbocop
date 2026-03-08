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

module Helpers
  describe FormHelper do
    describe "#sort_link" do
      subject { helper
        .sort_link(
          :name
        )
      }
      it { is_expected.to be_truthy }
    end
  end
end

RSpec.describe LazyModel do
  subject { described_class.new(&block) }
  let(:block) { proc { register_option(:parameter) } } # inline comment line should be skipped

  it { expect(subject).to be_truthy }
end

# Whitespace-only separator lines should count as blank.
RSpec.describe WhitespaceSeparatorAfterSubject do
  subject { described_class.new }
  
  let(:params) { foo }
end
