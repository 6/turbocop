shared_context 'foo' do
  let(:foo) { :bar }

  it 'performs actions' do
  end
end

shared_examples 'bar' do
  subject(:foo) { 'foo' }
  let(:bar) { :baz }
  before { initialize }

  it 'works' do
  end
end

shared_context 'empty' do
end

# shared_examples with let + it_behaves_like (example inclusions count as examples)
shared_examples 'literals that are frozen' do |o|
  let(:prefix) { o }

  it_behaves_like 'immutable objects', '[1, 2, 3]'
  it_behaves_like 'immutable objects', '%w(a b c)'
end

# shared_examples with include_examples
shared_examples 'mixed' do
  let(:x) { 1 }
  include_examples 'some examples'
end

# shared_context with describe containing before (nested context setup)
shared_context "delete is noop" do
  describe "the :delete action" do
    before(:each) do
      @info = []
    end

    it "deletes the resource" do
    end
  end
end

# shared_context with nested context/before inside describe
shared_context "callbacks" do
  describe "when callback is declared" do
    before(:each) do
      @called = nil
    end

    it "calls the callback" do
    end
  end
end

# shared_context with context containing let (nested setup)
shared_context "with permissions" do
  context "for admin" do
    let(:user) { create(:admin) }

    it "succeeds" do
    end
  end
end
