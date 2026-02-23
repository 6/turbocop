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

# Method calls named `context` or `it` that are NOT RSpec blocks
# should not be flagged (no block attached)
describe '#open?' do
  it "is consistent regardless of order" do
    items = records.first(10)
    results = 100.times.map { |n| subject.open?(context(75, :some_feature, items.shuffle)) }
    expect(results.uniq).to eq([true])
  end
end
