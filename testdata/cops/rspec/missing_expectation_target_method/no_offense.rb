RSpec.describe 'test' do
  it 'uses to' do
    expect(something).to be_a Integer
  end

  it 'uses not_to' do
    expect(something).not_to eq(42)
  end

  it 'uses to_not' do
    expect(something).to_not be_nil
  end

  it 'uses is_expected.to' do
    is_expected.to eq 42
  end

  it 'allows void expect' do
    expect(something)
  end
end
