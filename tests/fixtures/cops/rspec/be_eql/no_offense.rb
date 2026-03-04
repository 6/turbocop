it { expect(foo).to eql('foo') }
it { expect(foo).to_not eql(1) }
it { expect(foo).not_to eql(1) }
it { expect(foo).to eq(1) }
it { expect(foo).to be(true) }

# .to with a failure message as second argument — not a simple matcher call
it { expect(bom).to eql(false), "Expected BOM to be false" }
it { expect(result).to eql(true), custom_message }
