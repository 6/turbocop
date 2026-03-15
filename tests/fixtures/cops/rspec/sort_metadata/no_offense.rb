describe 'Something', :a, :b do
  it 'works' do
    expect(true).to eq(true)
  end
end

it 'Something', :a, :b, baz: true, foo: 'bar' do
  expect(1).to eq(1)
end

context 'Something', baz: true, foo: 'bar' do
  it 'has sorted hash keys' do
    expect(result).to be_valid
  end
end

# Block-argument style (&proc) should not be flagged: RuboCop's on_block
# only fires for BlockNode, not BlockArgumentNode
it 'Something', cli: true, visual: true, if: condition, &(proc do
end)

# Hooks with sorted metadata should not be flagged
RSpec.configure do |c|
  c.before(:each, :a, :b) { freeze_time }
  c.after(:each, baz: true, foo: 'bar') { travel_back }
end
