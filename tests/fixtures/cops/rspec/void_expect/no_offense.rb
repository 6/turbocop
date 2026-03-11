it 'something' do
  expect(something).to be 1
end
it 'something' do
  expect(something).not_to eq(2)
end
it 'something' do
  expect { something }.to raise_error(StandardError)
end
it 'something' do
  MyObject.expect(:foo)
end
# Void expect outside example block should not be flagged
describe 'something' do
  expect(something)
end
# Void expect at top level should not be flagged
expect(something)
# Void expect in shared_context should not be flagged
shared_context 'setup' do
  expect(something)
end
# Void expect in helper method should not be flagged
def helper
  expect(something)
end
# Sole expect inside conditional branch is NOT void per RuboCop
# (parent is if_type, not begin_type or block_type)
it 'conditional' do
  if condition
    expect(result)
  end
end
it 'unless branch' do
  unless condition
    expect(result)
  end
end
it 'ternary' do
  condition ? expect(a) : expect(b)
end
it 'case when sole' do
  case value
  when :one
    expect(result)
  end
end
# Modifier if/unless: expect is sole child of the if node
it 'modifier if' do
  expect(result) if condition
end
it 'modifier unless' do
  expect(result) unless condition
end
