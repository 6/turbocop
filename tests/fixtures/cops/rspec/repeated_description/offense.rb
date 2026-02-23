describe 'doing x' do
  it "does x" do
  ^^^^^^^^^^ RSpec/RepeatedDescription: Don't repeat descriptions within an example group.
  end

  it "does x" do
  ^^^^^^^^^^ RSpec/RepeatedDescription: Don't repeat descriptions within an example group.
  end
end

describe 'doing y' do
  it "does y" do
  ^^^^^^^^^^ RSpec/RepeatedDescription: Don't repeat descriptions within an example group.
  end

  context 'during some use case' do
    it "does y" do
    end
  end

  it "does y" do
  ^^^^^^^^^^ RSpec/RepeatedDescription: Don't repeat descriptions within an example group.
  end
end
