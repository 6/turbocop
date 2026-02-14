it { is_expected.to be_truthy }
it { is_expected.to eq(1) }
specify { expect(sqrt(4)).to eq(2) }
specify do
  is_expected.to be_truthy
end
it { are_expected.to be_falsy }
specify 'has a description' do
  expect(subject).to be_valid
end
