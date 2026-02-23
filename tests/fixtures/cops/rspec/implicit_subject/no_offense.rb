it { is_expected.to be_good }
it { should be_good }
it 'checks the subject' do
  expect(subject).to be_good
end
it 'checks negation' do
  expect(subject).not_to be_bad
end
expect(something).to eq(42)
