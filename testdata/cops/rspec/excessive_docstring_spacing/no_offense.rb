describe '#mymethod' do
  it 'does something' do
    expect(true).to eq(true)
  end
end

context 'when doing something' do
  it 'finds no should here' do
    expect(result).to eq(42)
  end
end

describe do
  it 'works without description' do
    expect(1 + 1).to eq(2)
  end
end

# Backslash-continued strings with single trailing space are NOT excessive
it 'corrects Layout/SpaceAroundOperators and Layout/ExtraSpacing ' \
   'offenses when using ForceEqualSignAlignment: true' do
  expect(true).to eq(true)
end

# Heredoc descriptions should be ignored entirely
it <<~DESC do
  does not remove
    another comments reports,
    another comments votes,
    another   comments image
DESC
  expect(true).to eq(true)
end

# Non-RSpec method calls with excessive whitespace should be ignored
it 'does something' do
  Foo.describe('  does something')
  Bar.context('  does something')
  Baz.it('  does something')
end
