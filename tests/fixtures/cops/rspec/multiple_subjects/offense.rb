describe 'hello there' do
  subject(:foo) { 1 }
  ^^^^^^^^^^^^^^^^^^^ RSpec/MultipleSubjects: Do not set more than one subject per example group
  subject(:bar) { 2 }
  ^^^^^^^^^^^^^^^^^^^ RSpec/MultipleSubjects: Do not set more than one subject per example group
  subject { 3 }
  ^^^^^^^^^^^^^ RSpec/MultipleSubjects: Do not set more than one subject per example group
  subject(:baz) { 4 }

  describe 'baz' do
    subject(:norf) { 1 }
  end
end

describe Doorkeeper::OpenidConnect::OAuth::PasswordAccessTokenRequest do
  if Gem.loaded_specs['doorkeeper'].version >= Gem::Version.create('5.5.1')
    subject { Doorkeeper::OAuth::PasswordAccessTokenRequest.new server, client, credentials, resource_owner, { nonce: '123456' } }
  else
    subject { Doorkeeper::OAuth::PasswordAccessTokenRequest.new server, client, resource_owner, { nonce: '123456' } }
  end
end

describe "#type_for_attribute" do
  if ::ActiveRecord::VERSION::STRING.to_f >= 4.2
    subject { SuperProduct }
  else
    subject { OtherProduct }
  end
end

describe 'Grape::EndpointExtension' do
  if Grape::Util.const_defined?('InheritableSetting')
    subject do
      Grape::Endpoint.new(
        Grape::Util::InheritableSetting.new,
        path: '/',
        method: 'foo'
      )
    end
  else
    subject do
      Grape::Endpoint.new
    end
  end
end