allow(foo).to receive(:bar).and_return('hello world')
allow(foo).to receive(:bar) { 'hello world' }
allow(foo).to receive(:bar).with(42).and_return('hello world')
allow(foo).to receive(:bar).with(42) { 'hello world' }
allow(foo).to receive_messages(foo: 42, bar: 777)
expect(foo).to have_received(:bar)
expect(foo).to receive(:bar)

# Intermediate methods (.twice/.once) break the message_expectation chain
expect(foo).to receive(:bar).twice.and_return('hello world')
expect(foo).to receive(:bar).and_return('hello world').once
expect(foo).to receive(:bar).once.and_return('hello world')
expect(foo).to receive(:call).twice.with(:arg).and_return(true)

# receive_message_chain with .with is NOT a configured response
expect(foo).to receive_message_chain(:bar, :baz).with(42)

# receive_message_chain without hash/block/configured_response is fine
expect(foo).to receive_message_chain(:bar, :baz)

# and_call_original/and_wrap_original take no arguments, so not flagged
expect(foo).to receive(:bar).and_call_original
expect(foo).to receive(:bar).and_wrap_original
