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

RSpec.describe StatusCodeMatcher do
  it_behaves_like "shared matcher" do
    subject(:matcher) { have_http_status(:not_found) }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterSubject: Add an empty line after `subject`.
    let(:code) { 404 }
  end
end

RSpec.shared_examples "logs request" do
  subject { log }
  ^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterSubject: Add an empty line after `subject`.
  it { is_expected.to include("Sending request") }
end
