let(:user_name) { 'Adam' }
let(:user_email) { 'adam@example.com' }
let(:age) { 20 }
let!(:record) { create(:record) }
subject(:result) { described_class.new }
let(:items_list) { [1, 2, 3] }

Mail.new do
  to 'some@email.com'
  subject 'testing premailer-rails'
end

if ENV['APPRAISAL_INITIALIZED']
  RSpec.describe 'wrapped by if' do
    let(:polyvalentEmployee) { Class.new }
  end
end

module Storages
  RSpec.describe OAuthUserToken do
    subject(:Authentication) { described_class }
  end
end

RSpec.describe "admin/api/v1/images" do
  path "/images" do
    post("Image create") do
      response(200, "successful") do
        let(:"") do
          {
            galleryId: 1,
            galleryType: "Model"
          }
        end
      end

      response(401, "unauthorized") do
        let(:"") { nil }
      end
    end
  end
end
