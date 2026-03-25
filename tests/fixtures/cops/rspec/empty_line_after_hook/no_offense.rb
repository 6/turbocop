RSpec.describe User do
  before { do_something }

  it { does_something }
end

RSpec.describe Post do
  after { cleanup }

  it { does_something }
end

RSpec.describe Comment do
  before { do_something }
end

# rubocop directive comment between hook and next statement counts as separator
RSpec.describe Widget do
  after(:all) do
    cleanup
  end
  # rubocop:enable RSpec/BeforeAfterAll

  let(:widget) { create(:widget) }
end

RSpec.describe Dataset do
  before { File.write(path, <<~CSV) }
    content,expected_output
    call this number for free money,true
  CSV

  let(:path) { "dataset.csv" }
end

class Minitest::Spec
  after :each do
    DatabaseCleaner.clean
  end

  include FactoryHelpers
end

# Whitespace-only separator lines should count as blank.
RSpec.describe WhitespaceSeparatorAfterHook do
  before { setup_context }

  it { expect(true).to be(true) }
end

# Hook with block argument (no actual block body) should not be flagged
RSpec.describe BlockArgHook do
  around(&rspec_around)
  subject { form.public_send(method_name) }
end

RSpec.describe BlockArgHookWithArgs do
  before(:context, &block)
  it { will_never_run }
end
