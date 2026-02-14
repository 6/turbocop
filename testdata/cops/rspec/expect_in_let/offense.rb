let(:foo) do
  expect(something).to eq 'foo'
  ^^^^^^ RSpec/ExpectInLet: Do not use `expect` in let
end
let(:bar) do
  is_expected.to eq 'bar'
  ^^^^^^^^^^^ RSpec/ExpectInLet: Do not use `is_expected` in let
end
let(:baz) do
  expect_any_instance_of(Something).to receive :foo
  ^^^^^^^^^^^^^^^^^^^^^^ RSpec/ExpectInLet: Do not use `expect_any_instance_of` in let
end
