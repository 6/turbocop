before do
  allow(Service).to receive(:foo).and_return(baz)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:bar).and_return(true)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:baz).and_return("x")
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end
