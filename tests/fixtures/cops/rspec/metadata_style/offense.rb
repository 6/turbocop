describe 'Something', a: true do
                      ^^^^^^^ RSpec/MetadataStyle: Use symbol style for metadata.
end

describe 'Something', a: true, b: true do
                               ^^^^^^^ RSpec/MetadataStyle: Use symbol style for metadata.
                      ^^^^^^^ RSpec/MetadataStyle: Use symbol style for metadata.
end

describe 'Something', :b, a: true do
                          ^^^^^^^ RSpec/MetadataStyle: Use symbol style for metadata.
end
