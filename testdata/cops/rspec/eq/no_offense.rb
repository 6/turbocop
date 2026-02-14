it { expect(foo).to eq true }

it { expect(bar).to eq(1) }

it { expect(baz).to be > 5 }

it { expect(foo).to be_truthy }

it { expect(bar).to eql(42) }
