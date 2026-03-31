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

  # Whitespace-only separator lines should count as blank.
  context 'with whitespace separator' do
    it { expect(true).to be(true) }
  end

  context 'after whitespace separator' do
    it { expect(true).to be(true) }
  end
end

# RSpec-prefixed groups with blank line separator are OK
RSpec.describe 'first' do
end

RSpec.describe 'second' do
end

# Postfix if/unless on end line — RuboCop skips these
module TestModule
  describe 'conditional group' do
    it 'works' do
    end
  end if ENV['RUN_TESTS']
  describe 'next group' do
    it 'also works' do
    end
  end
end

# Describe block as last child inside each loop
items.each do |item|
  describe item do
    it { expect(item).not_to be_nil }
  end
end

# Describe used as expression value (assigned to variable) — not a standalone group
RSpec.describe 'parent' do
  it "creates a group" do
    group = RSpec.describe { include SomeMixin }
    expect(group).not_to be_nil
  end
end

# Example group followed by __END__ — RuboCop treats __END__ as end-of-file
describe 'before __END__' do
  describe 'inner' do
    it 'works' do
    end
  end
end
__END__

This is data after __END__, not Ruby code.
