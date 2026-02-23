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

  # Context followed by else/elsif is OK (part of enclosing if/else)
  if some_condition
    context 'when condition' do
      it 'works' do
        expect(1).to eq(1)
      end
    end
  else
    context 'when other' do
      it 'also works' do
        expect(2).to eq(2)
      end
    end
  end
end
