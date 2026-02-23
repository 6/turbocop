describe '#foo' do
  it 'does something' do
    expect(true).to eq(true)
  end
end

context 'when foo is bar' do
  it 'returns the value' do
    expect(result).to eq(42)
  end
end

describe Foo do
  it 'works' do
    expect(subject).to be_valid
  end
end
