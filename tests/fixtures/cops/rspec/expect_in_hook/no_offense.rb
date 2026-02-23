before do
  allow(something).to receive(:foo)
end
after do
  cleanup_resources
end
it do
  expect(something).to eq('foo')
  is_expected.to eq('bar')
end
before do
end
