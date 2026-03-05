before do
  allow(Service).to receive(:foo).and_return(baz)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:bar).and_return(true)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:baz).and_return("x")
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end
# Stubs on local variable
before do
  allow(user).to receive(:foo).and_return(baz)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(user).to receive(:bar).and_return(qux)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end
# Non-consecutive stubs with other statements between them
before do
  allow(Service).to receive(:foo).and_return(1)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  calling_some_method
  calling_another_method
  allow(Service).to receive(:bar).and_return(2)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end
# Nested block (RSpec.describe > before)
RSpec.describe do
  before do
    allow(X).to receive(:foo).and_return(1)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
    allow(X).to receive(:bar).and_return(2)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
    allow(Y).to receive(:foo).and_return(3)
  end
end
# Group with duplicates: only non-duplicate stubs should be reported
before do
  allow(Service).to receive(:foo).and_return(bar)
  allow(Service).to receive(:bar).and_return(qux)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:foo).and_return(qux)
  allow(Service).to receive(:baz).and_return(qux)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end
