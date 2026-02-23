describe User do
  subject { User }

  let(:a) { a }
  let!(:b) { b }
  let(:c) { c }

  it { expect(subject.foo).to eq(a) }
end

describe Post do
  let(:x) { 1 }
  let(:y) { 2 }

  it { expect(x + y).to eq(3) }
end
