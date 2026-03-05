# Normal example groups and examples with bodies
describe 'test' do; end
context 'test' do; end
it 'test' do; end
example 'test' do; end
specify do; end
feature 'test' do; end
example_group 'test' do; end

# skip: false is not pending
it 'test', skip: false do; end

# :skip symbol as a matcher argument should not be flagged
it 'returns skip action' do
  expect(applier.action).to eq(:skip)
end

# :pending symbol as a matcher argument should not be flagged
it 'returns pending status' do
  expect(result.status).to eq(:pending)
end

# skip: keyword in non-RSpec method call should not be flagged
create(:record, skip: true)

# Method called pending on a receiver - not an RSpec pending call
subject { Project.pending }

# it as block parameter (Ruby 3.4+) - no args = not an example
expect(
  foo.map { it.reverse }
).to include(:bar)
