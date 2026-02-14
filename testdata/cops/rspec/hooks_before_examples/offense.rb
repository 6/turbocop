RSpec.describe User do
  it { is_expected.to be_valid }
  before { setup }
  ^^^^^^^^^^^^^^^^ RSpec/HooksBeforeExamples: Move `before` above the examples in the group.
end

RSpec.describe Post do
  it { is_expected.to be_present }
  after { cleanup }
  ^^^^^^^^^^^^^^^^^ RSpec/HooksBeforeExamples: Move `after` above the examples in the group.
end

RSpec.describe Comment do
  context 'nested' do
    it { is_expected.to work }
  end
  around { |test| test.run }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/HooksBeforeExamples: Move `around` above the examples in the group.
end
