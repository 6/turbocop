RSpec.describe User do
  let(:params) { foo }

  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
end

RSpec.describe Post do
  before { setup }

  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `before` declarations.
end

RSpec.describe Comment do
  it { is_expected.to be_present }

  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `it` declarations.
end
