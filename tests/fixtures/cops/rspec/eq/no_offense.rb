it { expect(foo).to eq true }

it { expect(bar).to eq(1) }

it { expect(baz).to be > 5 }

it { expect(foo).to be_truthy }

it { expect(bar).to eql(42) }

# Legacy .should syntax - be has a receiver
it { result.should.be == 1 }
it { value.should.not.be == 2 }
it { obj.should.be == "hello" }
