let(:foo) { 'bar' }
let(:baz) do
  compute_something
end
it do
  expect(something).to eq 'foo'
  is_expected.to eq 'bar'
  expect_any_instance_of(Something).to receive :foo
end
let(:empty) {}
