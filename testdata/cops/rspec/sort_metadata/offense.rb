describe 'Something', :b, :a do
                      ^^^^^^ RSpec/SortMetadata: Sort metadata alphabetically.
end

context 'Something', foo: 'bar', baz: true do
                     ^^^^^^^^^^^^^^^^^^^^^ RSpec/SortMetadata: Sort metadata alphabetically.
end

it 'Something', :b, :a, baz: true, foo: 'bar' do
                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SortMetadata: Sort metadata alphabetically.
end
