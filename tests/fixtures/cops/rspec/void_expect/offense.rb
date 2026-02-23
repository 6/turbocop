it 'something' do
  expect(something)
  ^^^^^^^^^^^^^^^^^ RSpec/VoidExpect: Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.
end
it 'another' do
  expect(another)
  ^^^^^^^^^^^^^^^ RSpec/VoidExpect: Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.
end
it 'third' do
  x = 1
  expect(x)
  ^^^^^^^^^ RSpec/VoidExpect: Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.
end
