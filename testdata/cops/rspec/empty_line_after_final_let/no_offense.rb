RSpec.describe User do
  let(:a) { a }
  let(:b) { b }

  it { expect(a).to eq(b) }
end

RSpec.describe Post do
  let(:x) { 1 }
  let(:y) { 2 }
end

RSpec.describe Comment do
  let(:foo) { 'bar' }

  specify { expect(foo).to eq('bar') }
end
