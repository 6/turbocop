before do
  allow(foo).to receive_message_chain(:one) { :two }
                ^^^^^^^^^^^^^^^^^^^^^ RSpec/SingleArgumentMessageChain: Use `receive` instead of calling `receive_message_chain` with a single argument.
end

before do
  allow(foo).to receive_message_chain("one") { :two }
                ^^^^^^^^^^^^^^^^^^^^^ RSpec/SingleArgumentMessageChain: Use `receive` instead of calling `receive_message_chain` with a single argument.
end

before do
  foo.stub_chain(:one) { :two }
      ^^^^^^^^^^ RSpec/SingleArgumentMessageChain: Use `stub` instead of calling `stub_chain` with a single argument.
end
