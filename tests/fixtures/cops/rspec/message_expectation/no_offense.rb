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
allow(items).to all receive(:process)
allow(items).to all(receive(:process).with(:arg))
# expect { block }.to receive: receiver is a block call, not a plain send
# RuboCop's NodePattern $(send nil? {:expect :allow} ...) requires a send node
# as receiver of .to; a block call produces a block node, not a send node
expect {
  subject
}.to receive(:stop)
