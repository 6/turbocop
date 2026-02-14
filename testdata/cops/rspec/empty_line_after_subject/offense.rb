RSpec.describe User do
  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterSubject: Add an empty line after `subject`.
  let(:params) { foo }
end

RSpec.describe Post do
  subject! { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterSubject: Add an empty line after `subject!`.
  let(:params) { foo }
end

RSpec.describe Comment do
  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterSubject: Add an empty line after `subject`.
  it { is_expected.to be_valid }
end
