describe 'Something', :a, :b do
  it 'works' do
    expect(true).to eq(true)
  end
end

describe 'Something' do
  it 'has no metadata' do
    expect(1).to eq(1)
  end
end

shared_examples 'something', :x, :y do
  it 'does stuff' do
    expect(result).to be_valid
  end
end
