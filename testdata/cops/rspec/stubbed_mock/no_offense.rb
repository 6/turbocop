allow(foo).to receive(:bar).and_return('hello world')
allow(foo).to receive(:bar) { 'hello world' }
allow(foo).to receive(:bar).with(42).and_return('hello world')
allow(foo).to receive(:bar).with(42) { 'hello world' }
allow(foo).to receive_messages(foo: 42, bar: 777)
expect(foo).to have_received(:bar)
expect(foo).to receive(:bar)
