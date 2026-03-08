RSpec.describe User do
  before { do_something }
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterHook: Add an empty line after `before`.
  it { does_something }
end

RSpec.describe Post do
  after { cleanup }
  ^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterHook: Add an empty line after `after`.
  it { does_something }
end

RSpec.describe Comment do
  around { |test| test.run }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterHook: Add an empty line after `around`.
  it { does_something }
end

RSpec.describe Item do
  context 'inside nested block' do
    resource '/items' do
      before { setup_data }
      ^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterHook: Add an empty line after `before`.
      get { 'ok' }
    end
  end
end

describe Widget do
  context 'with options' do
    scope :active do
      before(:host_name => 'example.com') { run_setup }
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterHook: Add an empty line after `before`.
      get('/') { 'ok' }
    end
  end
end

describe Service do
  context 'after hook in nested block' do
    scope '/' do
      after { cleanup }
      ^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterHook: Add an empty line after `after`.
      get('/') { 'ok' }
    end
  end
end

RSpec.describe Credentials do
  before { write_file(".chef/credentials", <<~TEXT) }
    [default]
    client_name = "testuser"
  TEXT
  ^^^^ RSpec/EmptyLineAfterHook: Add an empty line after `before`.
  it { does_something }
end

RSpec.describe Profile do
  # rubocop:disable RSpec/Foo
  before { setup_profile }
  # rubocop:enable RSpec/Foo
  ^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterHook: Add an empty line after `before`.
  let(:profile) { :default }
end

class Minitest::Spec
  after :each do
    DatabaseCleaner.clean
  end
  ^^^ RSpec/EmptyLineAfterHook: Add an empty line after `after`.
  include FactoryHelpers
end
