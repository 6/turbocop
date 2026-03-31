describe 'Something' do
end
describe 'Something', { a: :b } do
end
context 'Something', foo: true do
end
it 'test' do
end
specify 'test' do
end
# Empty hash as subject (first argument), not metadata
describe({}) do
end
# Empty hash as middle argument, not metadata
example(name.to_s, {}, caller(0)[1])
# Empty hash without a block — not metadata (RuboCop requires a block)
example :ExampleA, { }
it 'Something', {}
