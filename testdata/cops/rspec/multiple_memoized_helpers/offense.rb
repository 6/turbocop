describe Foo do
^^^^^^^^^^^^^^^ RSpec/MultipleMemoizedHelpers: Example group has too many memoized helpers [6/5]
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }
  let(:d) { 4 }
  let(:e) { 5 }
  let(:f) { 6 }

  it { expect(a + b + c + d + e + f).to eq(21) }
end

describe Bar do
^^^^^^^^^^^^^^^ RSpec/MultipleMemoizedHelpers: Example group has too many memoized helpers [7/5]
  let(:x) { 'x' }
  let(:y) { 'y' }
  let(:z) { 'z' }
  let(:w) { 'w' }
  let(:v) { 'v' }
  let!(:u) { 'u' }
  let!(:t) { 't' }

  it { expect(x).to eq('x') }
end

describe Baz do
^^^^^^^^^^^^^^^ RSpec/MultipleMemoizedHelpers: Example group has too many memoized helpers [6/5]
  let(:one) { 1 }
  let(:two) { 2 }
  let(:three) { 3 }
  let(:four) { 4 }
  let(:five) { 5 }
  let(:six) { 6 }

  it { expect(one).to be(1) }
end

# Nested context inherits parent lets: 4 + 2 = 6
describe Qux do
  let(:a) { 1 }
  let(:b) { 2 }
  let(:c) { 3 }
  let(:d) { 4 }

  context 'nested' do
  ^^^^^^^^^^^^^^^^^^^ RSpec/MultipleMemoizedHelpers: Example group has too many memoized helpers [6/5]
    let(:e) { 5 }
    let(:f) { 6 }

    it { expect(true).to be true }
  end
end
