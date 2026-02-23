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

# Nested context: 3 parent + 2 nested = 5, exactly at the Max
describe Baz do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }

  context 'nested stays under limit' do
    let(:d) { 4 }
    let(:e) { 5 }

    it { expect(true).to be true }
  end
end

# Overriding lets in child context do not increase count
describe Overrides do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }
  let(:d) { 4 }
  let(:e) { 5 }

  context 'redefines some parent lets' do
    let(:a) { 10 }
    let(:b) { 20 }

    it { expect(a).to eq(10) }
  end
end
