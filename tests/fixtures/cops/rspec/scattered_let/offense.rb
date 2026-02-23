RSpec.describe User do
  let(:a) { a }
  it { expect(subject.foo).to eq(a) }
  let(:b) { b }
  ^^^^^^^^^^^^^ RSpec/ScatteredLet: Group all let/let! blocks in the example group together.
end

describe Post do
  let(:x) { 1 }
  let(:y) { 2 }
  it { expect(x + y).to eq(3) }
  let(:z) { 3 }
  ^^^^^^^^^^^^^ RSpec/ScatteredLet: Group all let/let! blocks in the example group together.
end

describe Comment do
  let!(:a) { create(:a) }
  it { expect(a).to be_valid }
  let!(:b) { create(:b) }
  ^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ScatteredLet: Group all let/let! blocks in the example group together.
end
