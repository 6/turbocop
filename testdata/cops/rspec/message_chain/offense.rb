allow(foo).to receive_message_chain(:one, :two)
              ^^^^^^^^^^^^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `receive_message_chain`.
allow(bar).to receive_message_chain(:a, :b, :c)
              ^^^^^^^^^^^^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `receive_message_chain`.
foo.stub_chain(:one, :two).and_return(:three)
    ^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `stub_chain`.
