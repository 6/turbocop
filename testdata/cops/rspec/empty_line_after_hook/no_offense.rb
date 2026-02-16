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
