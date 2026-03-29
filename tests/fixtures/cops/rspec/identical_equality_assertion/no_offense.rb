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

  it 'allows extra message arguments on matcher runners' do
    expect(3).to eq(3), :not_a_string
    expect('1').to(eq('1'), '1 should equal 1')
  end
end
