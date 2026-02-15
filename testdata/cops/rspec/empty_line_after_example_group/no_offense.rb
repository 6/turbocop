RSpec.describe Foo do
  describe '#bar' do
  end

  describe '#baz' do
  end

  context 'first' do
  end

  context 'second' do
  end

  # Comment followed by end is OK
  context 'with comment before end' do
    it { expect(1).to eq(1) }
  end
  # TODO: add more tests
end
