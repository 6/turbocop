expect(foo).to have_received(:bar)
expect(foo).not_to have_received(:bar)
allow(foo).to receive(:bar)
allow(foo).to receive(:baz).and_return(true)
expect(result).to eq(42)
expect(foo).to be_truthy
