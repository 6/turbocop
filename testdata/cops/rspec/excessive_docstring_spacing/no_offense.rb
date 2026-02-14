describe '#mymethod' do
  it 'does something' do
    expect(true).to eq(true)
  end
end

context 'when doing something' do
  it 'finds no should here' do
    expect(result).to eq(42)
  end
end

describe do
  it 'works without description' do
    expect(1 + 1).to eq(2)
  end
end
