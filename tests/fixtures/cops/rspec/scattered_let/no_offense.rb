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

# Shared groups are not example groups — ScatteredLet only runs in example groups
shared_examples "scattered in shared examples" do
  let(:a) { 1 }
  it { expect(a).to eq(1) }
  let(:b) { 2 }
end

shared_examples_for "scattered in shared examples for" do
  let(:x) { 1 }
  before { setup }
  let(:y) { 2 }
end

shared_context "scattered in shared context" do
  let(:item) { create(:item) }
  before { prepare }
  let(:other) { create(:other) }
end

# let with &proc block argument counts as a let declaration — no scatter here
describe Connection do
  let(:connection) { described_class.new }
  let :fresh_connection, &NEW_PG_CONNECTION
  it { expect(connection).to be_valid }
end

# RuboCop 1.84.2 + rubocop-rspec 3.9.0 drops a scattered bare block_pass let
describe HookedConnection do
  let(:connection) { described_class.new }
  before { setup }
  let :fresh_connection, &NEW_PG_CONNECTION
end

# Corpus FP: nested example group inside an iterator with a scattered bare block_pass let
describe Que::Connection do
  QUE_POOLS.each do |name, pool|
    describe "with a #{name} connection pool" do
      let(:connection) { @connection }

      around do |&block|
        super() do
          pool.checkout do |conn|
            @connection = conn
            block.call
          end
        end
      end

      let :fresh_connection, &NEW_PG_CONNECTION

      describe ".wrap()" do
        it { expect(connection).to be_a(Object) }
      end
    end
  end
end

# Once RuboCop hits the scattered bare block_pass let, it stops reporting later lets in the group
describe LaterRegularLet do
  let(:connection) { described_class.new }
  before { setup }
  let :fresh_connection, &NEW_PG_CONNECTION
  let(:other) { create(:other) }
end
