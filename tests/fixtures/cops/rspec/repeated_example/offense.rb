describe 'doing x' do
  it "does x" do
  ^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 6.
    expect(foo).to be(bar)
  end

  it "does y" do
  ^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 2.
    expect(foo).to be(bar)
  end
end

describe 'doing y' do
  its(:x) { is_expected.to be_present }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 13.
  its(:x) { is_expected.to be_present }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 12.
end

# Different formatting but same AST body — should still be detected as duplicates
describe 'mixed formatting' do
  it "does x" do
  ^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 21.
    expect(foo).to be(bar)
  end
  it "does y" do expect(foo).to be(bar); end
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 18.
end

# One-liner examples with duplicates
describe 'one-liners' do
  it { is_expected.to be_valid }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 27, 28.
  it { is_expected.to be_valid }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 26, 28.
  it { is_expected.to be_valid }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 26, 27.
end

# Multiline vs single-line with same body
describe 'multiline vs brace' do
  it "multiline" do
  ^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 36.
    expect(foo).to eq(bar)
  end
  it("single line") { expect(foo).to eq(bar) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 33.
end

# Four duplicates with different descriptions
describe 'four dupes' do
  it "first" do
  ^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 44, 47, 50.
    expect(foo).to be(bar)
  end
  it "second" do
  ^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 41, 47, 50.
    expect(foo).to be(bar)
  end
  it "third" do
  ^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 41, 44, 50.
    expect(foo).to be(bar)
  end
  it "fourth" do
  ^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 41, 44, 47.
    expect(foo).to be(bar)
  end
end
