describe 'doing x' do
  it "does x" do
  end

  context 'in a certain use case' do
    it "does x" do
    end
  end
end

describe 'doing y' do
  it { foo }
  it { bar }
end

# shared_examples are not checked for repeated descriptions
shared_examples 'default locale' do
  it 'sets available and preferred language' do
    1
  end

  it 'sets available and preferred language' do
    2
  end
end

# its() with same argument but different block body is NOT a repeat
describe 'doing z' do
  its(:name) { is_expected.to be_present }
  its(:name) { is_expected.to be_blank }
end
