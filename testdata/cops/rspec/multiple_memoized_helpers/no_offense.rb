describe Foo do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }
  let(:d) { 4 }
  let(:e) { 5 }

  it { expect(a + b + c + d + e).to eq(15) }
end

describe Bar do
  let(:x) { 'x' }

  it { expect(x).to eq('x') }
end
