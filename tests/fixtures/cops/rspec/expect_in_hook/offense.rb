before do
  expect(something).to eq('foo')
  ^^^^^^ RSpec/ExpectInHook: Do not use `expect` in `before` hook
end
after do
  is_expected.to eq('bar')
  ^^^^^^^^^^^ RSpec/ExpectInHook: Do not use `is_expected` in `after` hook
end
around do
  expect_any_instance_of(Something).to receive(:foo)
  ^^^^^^^^^^^^^^^^^^^^^^ RSpec/ExpectInHook: Do not use `expect_any_instance_of` in `around` hook
end
