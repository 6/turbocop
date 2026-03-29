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

shared_examples 'sortable' do
  let(:records) { create_list(:record, 3) }

  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
end

shared_context 'with authentication' do
  before { sign_in(user) }

  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `before` declarations.
end

RSpec.describe User do
  describe '#valid?' do
    let(:params) { foo }

    subject { described_class.new }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
  end
end

RSpec.describe User do
  shared_examples_for 'a model' do
    it { is_expected.to be_present }
  end

  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `shared_examples_for` declarations.
end

RSpec.describe User do
  let!(:record) { create(:record) }

  subject! { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let!` declarations.
end

RSpec.describe User do
  it_behaves_like 'sortable' do
    let(:params) { foo }

    subject { described_class.new }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
  end
end

RSpec.describe User do
  include_context 'with setup' do
    before { setup }

    subject { described_class.new }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `before` declarations.
  end
end

RSpec.describe User do
  with_feature_flag(:new_ui) do
    let(:params) { foo }

    subject { described_class.new }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
  end
end

RSpec.describe User do
  custom_setup do
    before { setup }

    subject { described_class.new }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `before` declarations.
  end
end

RSpec.describe User do
  let(:user, &args[:build_user])

  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
end

RSpec.shared_examples_for 'a model' do
  let(:params) { foo }

  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
end

RSpec.shared_context 'with setup' do
  before { setup }

  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `before` declarations.
end

RSpec.feature 'User management' do
  let(:admin) { create(:admin) }

  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
end

RSpec.describe User do
  items.each do |item|
    context "with #{item}" do
      let(:record) { create(:record, item: item) }

      subject { described_class.new }
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
    end
  end
end

RSpec.describe User do
  include_context 'shared setup'

  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `include_context` declarations.
end

RSpec.describe User do
  it_behaves_like 'sortable'

  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `it_behaves_like` declarations.
end

RSpec.describe User do
  context "with items" do
    [
      ["admin", "viewer"],
      ["editor", "viewer"]
    ].each do |role_a, role_b|
      context "when #{role_a} and #{role_b}" do
        include_context role_a
        include_context role_b

        let(:record) { create(:record) }

        subject { described_class.new }
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `include_context` declarations.
      end
    end
  end
end

RSpec.describe User do
  describe "with name" do
    records.each do |(status, blocked), expectation|
      describe "with status" do
        let(:user) { build_user(status, blocked) }

        subject(:user_status) { full_user_status(user, true) }
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
      end
    end
  end
end

RSpec.shared_examples "upload resource" do
  describe "POST /prepare" do
    let(:params) { build(:params) }

    def request!
      post path, params
    end

    subject(:response) { last_response }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
  end
end

RSpec.shared_examples "attachment API" do
  it_behaves_like "upload" do
    let(:request_path) { "/api/v3/attachments" }
  end

  subject(:response) { last_response }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `it_behaves_like` declarations.
end

RSpec.shared_examples_for "multiple errors" do
  let(:errors) { JSON.parse(last_response.body) }

  subject { errors.inject({}) { |h, d| h.merge(d) } }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
end

RSpec.describe User do
  if enabled?
    describe "protected methods" do
      let(:params) { build(:params) }

      subject { described_class.new(params) }
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
    end
  end
end

RSpec.describe User do
  unless ENV["CI"]
    context "with local setup" do
      after(:each) { subject.close }

      let(:host) { "127.0.0.1" }
      subject { described_class.new(hostname: host) }
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `after` declarations.
    end
  end
end

RSpec.describe User do
  unless skip_tests?
    context "with config" do
      let(:config) { build(:config) }
      subject { described_class.new(config) }
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
    end
  end
end

RSpec.describe User do
  if enabled?
    subject { build_user(server) }
    ^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
  else
    subject { build_user(client) }
    ^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
  end

  let(:server) { :server }
  let(:client) { :client }
end

RSpec.describe "Endpoint" do
  if enabled?
    subject do
    ^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
      build_endpoint
    end
  else
    subject do
    ^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.
      fallback_endpoint
    end
  end

  let(:serializer) { :serializer }
end

RSpec.describe User do
  describe "#evaluate" do
    let(:predicate) { described_class.unconditional }

    def self.specify_claim
      subject(:evaluate) { predicate.method(:evaluate) }
      ^ RSpec/LeadingSubject: Declare `subject` above any other `let` declarations.

      context "with claim" do
        let(:created) { Time.now }
      end
    end
  end
end
