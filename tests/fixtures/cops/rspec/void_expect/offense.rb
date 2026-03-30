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
# Block form of expect (expect { ... })
it 'block form' do
  expect { something }
  ^^^^^^^^^^^^^^^^^^^^ RSpec/VoidExpect: Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.
end
# Block form as sole statement
it 'block form sole' do
  expect{something}
  ^^^^^^^^^^^^^^^^^ RSpec/VoidExpect: Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.
end
# Nested inside describe/context
describe Foo do
  context 'bar' do
    it 'nested void expect' do
      expect(result)
      ^^^^^^^^^^^^^^ RSpec/VoidExpect: Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.
    end
  end
end
# Inside aggregate_failures
it 'test' do
  aggregate_failures do
    expect(one)
    ^^^^^^^^^^^ RSpec/VoidExpect: Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.
  end
end
# Multi-statement if branch: expect's parent is begin_type? -> void
it 'multi-statement if' do
  if condition
    setup
    expect(result)
    ^^^^^^^^^^^^^^ RSpec/VoidExpect: Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.
  end
end
# Parenthesized expect with .to is still void per RuboCop
# (parens create a begin node, making the expect's parent begin_type?)
it 'parenthesized chained' do
  (expect something).to be 1
   ^^^^^^^^^^^^^^^^ RSpec/VoidExpect: Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.
end
it 'parenthesized chained not_to' do
  (expect result).not_to eq(2)
   ^^^^^^^^^^^^^ RSpec/VoidExpect: Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.
end

# Chained block-form expect is still void when the block body is a bare `expect`
# that is structurally identical to the outer expect send.
it 'nested bare expect in block expectation' do
  expect {
  ^^^^^^ RSpec/VoidExpect: Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.
    expect
    ^^^^^^ RSpec/VoidExpect: Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.
  }.to raise_error(ArgumentError)
end
