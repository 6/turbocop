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
