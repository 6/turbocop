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

module Spree
  describe LegacyUser do
    let(:user) { create(:user) }
    before { setup }
    subject { described_class.new }
  end
end

require 'spec_helper'
module Berkshelf
  describe ChefRepoUniverse do
    let(:fixture) { nil }
    subject { described_class.new(fixture) }
  end
end

class Configuration
  describe Server do
    let(:server) { build(:server) }
    subject { described_class.new }
  end
end

shared_examples 'sortable' do
  subject { described_class.new }
  let(:records) { create_list(:record, 3) }
end

shared_context 'with authentication' do
  subject { described_class.new }
  before { sign_in(user) }
end

RSpec.describe User do
  describe '#valid?' do
    subject { described_class.new }
    let(:params) { foo }
  end
end

RSpec.describe User do
  let(:foo) { 'bar' }

  it_behaves_like 'a good citizen' do
    subject { described_class.new }
  end
end

RSpec.describe User do
  it "doesn't mind me calling a method called subject in the test" do
    let(foo)
    subject { bar }
  end
end

RSpec.describe User do
  with_feature_flag(:new_ui) do
    subject { described_class.new }
    let(:params) { foo }
  end
end

RSpec.shared_examples_for 'a model' do
  subject { described_class.new }
  let(:params) { foo }
end

RSpec.shared_context 'with setup' do
  subject { described_class.new }
  before { setup }
end

RSpec.feature 'User management' do
  subject { described_class.new }
  let(:admin) { create(:admin) }
end

RSpec.describe User do
  items.each do |item|
    context "with #{item}" do
      subject { described_class.new }
      let(:record) { create(:record, item: item) }
    end
  end
end

RSpec.shared_examples "upload resource" do
  describe "POST /prepare" do
    subject(:response) { last_response }

    let(:params) { build(:params) }

    def request!
      post path, params
    end
  end
end

RSpec.describe User do
  describe "with name" do
    records.each do |(status, blocked), expectation|
      describe "with status" do
        subject(:user_status) { full_user_status(user, true) }
        let(:user) { build_user(status, blocked) }
      end
    end
  end
end

RSpec.shared_examples "attachment API" do
  subject(:response) { last_response }

  it_behaves_like "upload" do
    let(:request_path) { "/api/v3/attachments" }
  end
end

RSpec.describe User do
  context "with items" do
    [
      ["admin", "viewer"],
      ["editor", "viewer"]
    ].each do |role_a, role_b|
      context "when #{role_a} and #{role_b}" do
        subject { described_class.new }
        include_context role_a
        include_context role_b
        let(:record) { create(:record) }
      end
    end
  end
end

RSpec.describe User do
  if enabled?
    describe "protected methods" do
      subject { described_class.new }
      let(:params) { build(:params) }
    end
  end
end

RSpec.describe User do
  unless ENV["CI"]
    context "with local setup" do
      subject { described_class.new }
      let(:host) { "127.0.0.1" }
    end
  end
end

# Subject referenced inside def method body (not a declaration)
RSpec.describe Grape::API::Helpers do
  subject { Class.new }
  def app
    subject
  end
  context 'with lack of expected params' do
    it 'returns missing error' do
    end
  end
end

# Hook with block_pass (not a real block) is NOT offending
RSpec.describe User do
  shared_examples "the default form builder" do |method_name, rspec_around: lambda(&:run)|
    around(&rspec_around)
    subject { described_class.new }
  end
end

# describe without a block is NOT a spec_group (not offending)
RSpec.describe User do
  describe 'available attributes' do
    {name: 'rspec'}.each do |attr, value|
      context attr do
        describe [attr]
        subject { super()[attr] }
        let(:attributes) { { attr => value } }
      end
    end
  end
end
