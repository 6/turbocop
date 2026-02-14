RSpec.describe User do
  let(:a) { a }
  let(:b) { b }
  ^^^^^^^^^^^^^ RSpec/EmptyLineAfterFinalLet: Add an empty line after the last `let`.
  it { expect(a).to eq(b) }
end

RSpec.describe Post do
  let(:x) { 1 }
  let!(:y) { 2 }
  ^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterFinalLet: Add an empty line after the last `let!`.
  it { expect(x + y).to eq(3) }
end

RSpec.describe Comment do
  let(:foo) { 'foo' }
  ^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterFinalLet: Add an empty line after the last `let`.
  specify { expect(foo).to eq('foo') }
end
