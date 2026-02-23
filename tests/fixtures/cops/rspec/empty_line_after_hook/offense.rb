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
