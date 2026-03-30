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

# Different quote styles should still match (same content)
describe 'quote normalization' do
  it 'handles invalid byte sequences' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedDescription: Don't repeat descriptions within an example group.
  end

  it "handles invalid byte sequences" do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedDescription: Don't repeat descriptions within an example group.
  end
end

describe 'iterator examples' do
  %i[foo bar].each do |type|
    it "does a thing #{type}" do
    ^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedDescription: Don't repeat descriptions within an example group.
    end

    it "does a thing #{type}" do
    ^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedDescription: Don't repeat descriptions within an example group.
    end
  end
end

describe 'empty descriptions' do
  specify "" do
  ^^^^^^^^^^^^^ RSpec/RepeatedDescription: Don't repeat descriptions within an example group.
  end

  specify "" do
  ^^^^^^^^^^^^^ RSpec/RepeatedDescription: Don't repeat descriptions within an example group.
  end
end
