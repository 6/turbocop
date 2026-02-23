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
