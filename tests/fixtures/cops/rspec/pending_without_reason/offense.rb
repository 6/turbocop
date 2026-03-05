RSpec.describe Foo do
  pending
  ^^^^^^^ RSpec/PendingWithoutReason: Give the reason for pending.
  skip
  ^^^^ RSpec/PendingWithoutReason: Give the reason for skip.
  xit 'something' do
  ^^^^^^^^^^^^^^^ RSpec/PendingWithoutReason: Give the reason for xit.
  end
  it 'does something', :pending do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/PendingWithoutReason: Give the reason for pending.
  end
  it 'does something', :skip do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/PendingWithoutReason: Give the reason for skip.
  end
  it 'does something', pending: true do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/PendingWithoutReason: Give the reason for pending.
  end
  it 'does something', skip: true do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/PendingWithoutReason: Give the reason for skip.
  end
  pending 'does something' do
  ^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/PendingWithoutReason: Give the reason for pending.
  end
  skip 'does something' do
  ^^^^^^^^^^^^^^^^^^^^^ RSpec/PendingWithoutReason: Give the reason for skip.
  end
  it 'does something' do
    pending
    ^^^^^^^ RSpec/PendingWithoutReason: Give the reason for pending.
  end
  it 'does something' do
    skip
    ^^^^ RSpec/PendingWithoutReason: Give the reason for skip.
  end
  context 'when something' do
    pending
    ^^^^^^^ RSpec/PendingWithoutReason: Give the reason for pending.
    skip
    ^^^^ RSpec/PendingWithoutReason: Give the reason for skip.
  end
end
