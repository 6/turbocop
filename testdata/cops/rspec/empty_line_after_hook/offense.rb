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
