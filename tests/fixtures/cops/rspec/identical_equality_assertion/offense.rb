RSpec.describe 'test' do
  it 'compares with eq' do
    expect(foo.bar).to eq(foo.bar)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IdenticalEqualityAssertion: Identical expressions on both sides of the equality may indicate a flawed test.
  end

  it 'compares with eql' do
    expect(foo.bar.baz).to eql(foo.bar.baz)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IdenticalEqualityAssertion: Identical expressions on both sides of the equality may indicate a flawed test.
  end

  it 'compares trivial constants' do
    expect(42).to eq(42)
    ^^^^^^^^^^^^^^^^^^^^ RSpec/IdenticalEqualityAssertion: Identical expressions on both sides of the equality may indicate a flawed test.
  end
end
