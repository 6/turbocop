it { is_expected.to be_good }
it { should be_good }
it 'checks the subject' do
  expect(subject).to be_good
end
it 'checks negation' do
  expect(subject).not_to be_bad
end
expect(something).to eq(42)
its(:title) { should eq 'hello' }
its(:name) { is_expected.to eq 'world' }
its(:quality) do
  is_expected.to be :high
end
its(:status) { should_not be_nil }
it 'uses some similar sounding methods' do
  expect(baz).to receive(:is_expected)
  baz.is_expected
  foo.should(deny_access)
end
