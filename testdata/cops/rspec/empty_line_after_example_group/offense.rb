RSpec.describe Foo do
  describe '#bar' do
  end
  ^^^ RSpec/EmptyLineAfterExampleGroup: Add an empty line after `describe`.
  describe '#baz' do
  end

  context 'first' do
  end
  ^^^ RSpec/EmptyLineAfterExampleGroup: Add an empty line after `context`.
  context 'second' do
  end

  shared_examples 'foo' do
  end
  ^^^ RSpec/EmptyLineAfterExampleGroup: Add an empty line after `shared_examples`.
  it 'works' do
  end
end
