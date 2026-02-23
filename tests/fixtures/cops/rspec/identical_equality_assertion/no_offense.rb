RSpec.describe 'test' do
  it 'compares different expressions' do
    expect(foo.bar).to eq(bar.foo)
  end

  it 'checks for whole expression' do
    expect(Foo.new(1).foo).to eql(Foo.new(2).bar)
  end

  it 'compares different values' do
    expect(result).to eq(expected)
  end

  it 'uses be matcher with different values' do
    expect(foo).to be(bar)
  end
end
