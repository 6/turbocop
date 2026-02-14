describe 'Something', :a, :b do
  it 'works' do
    expect(true).to eq(true)
  end
end

it 'Something', :a, :b, baz: true, foo: 'bar' do
  expect(1).to eq(1)
end

context 'Something', baz: true, foo: 'bar' do
  it 'has sorted hash keys' do
    expect(result).to be_valid
  end
end
