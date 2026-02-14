expect(foo).not_to receive(:bar)

allow(foo).to receive(:bar).never

allow(foo).to receive(:bar).with(1).never

foo.never

expect(foo).to never_call_this_method
