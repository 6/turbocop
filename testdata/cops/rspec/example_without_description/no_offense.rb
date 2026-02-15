it 'is good' do
  expect(subject).to be_good
end
specify 'is good' do
  expect(subject).to be_good
end
it { expect(subject).to be_good }
specify { expect(subject).to be_good }
it { is_expected.to be_truthy }
it do
  expect(subject).to be_good
end
