allow(foo).to receive_message_chain(:one, :two)
              ^^^^^^^^^^^^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `receive_message_chain`.
allow(bar).to receive_message_chain(:a, :b, :c)
              ^^^^^^^^^^^^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `receive_message_chain`.
foo.stub_chain(:one, :two).and_return(:three)
    ^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `stub_chain`.

chains.add(:method_name, stub_chain)
                         ^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `stub_chain`.

chains.add(:method_name, stub_chain)
                         ^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `stub_chain`.

chains.add(:method_name, stub_chain)
                         ^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `stub_chain`.

chains.add(:method_name, stub_chain)
                         ^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `stub_chain`.

expect(chains[:method_name]).to eq([stub_chain])
                                    ^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `stub_chain`.

chains.add(:method_name, stub_chain)
                         ^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `stub_chain`.

expect(chains[:method_name]).to eq([stub_chain, another_stub_chain])
                                    ^^^^^^^^^^ RSpec/MessageChain: Avoid stubbing using `stub_chain`.
