describe 'Something', :b, :a do
                      ^^^^^^ RSpec/SortMetadata: Sort metadata alphabetically.
end

context 'Something', foo: 'bar', baz: true do
                     ^^^^^^^^^^^^^^^^^^^^^ RSpec/SortMetadata: Sort metadata alphabetically.
end

it 'Something', :b, :a, baz: true, foo: 'bar' do
                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SortMetadata: Sort metadata alphabetically.
end

RSpec.configure do |c|
  c.before(:each, :b, :a) { freeze_time }
                  ^^^^^^ RSpec/SortMetadata: Sort metadata alphabetically.
  c.after(:each, foo: 'bar', baz: true) { travel_back }
                 ^^^^^^^^^^^^^^^^^^^^^ RSpec/SortMetadata: Sort metadata alphabetically.
end
