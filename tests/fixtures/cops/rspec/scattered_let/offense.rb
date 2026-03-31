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

RSpec.feature "Widgets" do
  let(:widget) { create(:widget) }
  it { expect(widget).to be_valid }
  let(:other) { create(:other) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ScatteredLet: Group all let/let! blocks in the example group together.
end

# parenthesized let with &block_pass is still reported when scattered
describe Connection do
  let(:connection) { described_class.new }
  before { setup }
  let(:fresh_connection, &NEW_PG_CONNECTION)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ScatteredLet: Group all let/let! blocks in the example group together.
end

# block_pass let in initial group, then scattered regular let
describe Service do
  let(:handler, &HANDLER_PROC)
  it { expect(handler).to be_valid }
  let(:other) { create(:other) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ScatteredLet: Group all let/let! blocks in the example group together.
end
