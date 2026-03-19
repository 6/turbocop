RSpec.describe Foo do
  pending
  ^^^^^^^ RSpec/PendingWithoutReason: Give the reason for pending.
  skip
  ^^^^ RSpec/PendingWithoutReason: Give the reason for skip.
  xit 'something' do
  ^^^^^^^^^^^^^^^ RSpec/PendingWithoutReason: Give the reason for xit.
  end
  xit
  ^^^ RSpec/PendingWithoutReason: Give the reason for xit.
end

RSpec.xdescribe 'top level skipped' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/PendingWithoutReason: Give the reason for skip.
  it 'does something' do
  end
end

RSpec.xcontext 'top level skipped context' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/PendingWithoutReason: Give the reason for skip.
  it 'does something' do
  end
end

RSpec.describe Foo do
  xdescribe 'nested skipped without RSpec receiver' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/PendingWithoutReason: Give the reason for skip.
    it 'does something' do
    end
  end
end
