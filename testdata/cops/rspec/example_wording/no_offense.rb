it 'does something' do
  expect(true).to eq(true)
end
it 'returns the correct value' do
  expect(1 + 1).to eq(2)
end
specify 'works correctly' do
  expect(subject).to be_valid
end
it 'is valid' do
  expect(user).to be_valid
end
it 'displays shoulder text' do
  expect(page).to have_content('shoulder')
end
