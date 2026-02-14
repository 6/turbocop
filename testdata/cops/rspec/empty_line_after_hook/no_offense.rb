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
