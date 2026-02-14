RSpec.describe Foo do
  it 'does this' do
  end
  ^^^ RSpec/EmptyLineAfterExample: Add an empty line after `it`.
  it 'does that' do
  end

  specify do
  end
  ^^^ RSpec/EmptyLineAfterExample: Add an empty line after `specify`.
  specify 'something else' do
  end

  it 'another example' do
  end
  ^^^ RSpec/EmptyLineAfterExample: Add an empty line after `it`.
  # a comment
  it 'yet another' do
  end
end
