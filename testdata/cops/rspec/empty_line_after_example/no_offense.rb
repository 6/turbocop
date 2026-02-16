RSpec.describe Foo do
  it 'does this' do
  end

  it 'does that' do
  end

  it { one }
  it { two }

  specify do
  end

  # Comment with blank line between it and next example is OK
  it 'does something' do
  end
  # rubocop:enable RSpec/SomeOtherCop

  it 'another thing' do
  end
end
