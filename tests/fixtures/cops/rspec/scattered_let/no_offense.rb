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
