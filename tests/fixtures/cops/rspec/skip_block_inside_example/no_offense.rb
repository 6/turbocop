it 'does something' do
  skip 'not yet implemented'
end
it 'works fine' do
  expect(true).to be true
end
skip 'not yet implemented' do
end
it 'another example' do
  pending 'work in progress' do
  end
end
specify 'also fine' do
  skip
end
