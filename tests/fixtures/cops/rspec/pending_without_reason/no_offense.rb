pending 'reason'
skip 'reason'
it 'does something', pending: 'reason' do
end
it 'does something', skip: 'reason' do
end
describe 'something', pending: 'reason' do
end

RSpec.describe Foo do
  it 'does something' do
    next skip
  end
end


# top-level RSpec.xdescribe with a reason in metadata is still no_offense
RSpec.describe Foo, skip: 'reason' do
end
