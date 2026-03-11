before do
  allow(Service).to receive(:foo).and_return(baz)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:bar).and_return(true)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:baz).and_return("x")
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end

# Stubs in a method body (def) should also be detected
def setup_stubs
  allow(Service).to receive(:name).and_return("test")
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:status).and_return(:active)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end

# Duplicate receive args: unique items should still be flagged
before do
  allow(Service).to receive(:foo).and_return(bar)
  allow(Service).to receive(:bar).and_return(qux)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:foo).and_return(qux)
  allow(Service).to receive(:baz).and_return(qux)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end

# Stubs with other statements between them
before do
  allow(Service).to receive(:alpha).and_return(1)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  call_something
  allow(Service).to receive(:beta).and_return(2)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end
