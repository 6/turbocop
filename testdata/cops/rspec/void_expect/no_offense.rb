it 'something' do
  expect(something).to be 1
end
it 'something' do
  expect(something).not_to eq(2)
end
it 'something' do
  expect { something }.to raise_error(StandardError)
end
it 'something' do
  MyObject.expect(:foo)
end
