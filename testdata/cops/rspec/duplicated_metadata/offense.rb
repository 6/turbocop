describe 'Something', :a, :a do
                          ^^ RSpec/DuplicatedMetadata: Avoid duplicated metadata.
end

it 'does something', :b, :b do
                         ^^ RSpec/DuplicatedMetadata: Avoid duplicated metadata.
end

shared_examples 'something', :c, :c do
                                 ^^ RSpec/DuplicatedMetadata: Avoid duplicated metadata.
end
