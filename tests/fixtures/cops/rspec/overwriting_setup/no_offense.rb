RSpec.describe User do
  let(:a) { a }

  context 'different' do
    let(:a) { b }
  end
end

RSpec.describe User do
  subject(:name) { a }

  let(:other) { b }
end

RSpec.describe User do
  subject(:foo) { a }
  callback = -> {}

  let(:foo, &callback)
end

shared_examples_for "parameterized setup" do
  let(:setting) { "one" }
  it_behaves_like "shared contract"

  let(:setting) { "two" }
  it_behaves_like "shared contract"
end
