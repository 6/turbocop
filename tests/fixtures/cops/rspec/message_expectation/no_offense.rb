allow(foo).to receive(:bar)
allow(foo).to receive(:baz).with(1)
allow(obj).to receive(:qux).and_return(true)
expect(foo).to eq(bar)
expect(foo).to have_received(:bar)
allow(foo).to receive(:bar).and_return(42)
expect(obj).not_to receive(:qux)
expect(obj).to_not receive(:qux)
# &proc block argument: RuboCop's NodePattern (send ... :to receive?) doesn't match
# when there's a block_pass child (extra argument in Parser AST)
expect(fake_request).to receive(:on).with(:close), &on_close
