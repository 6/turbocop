it { expect(foo).to be(1) }
it { expect(foo).not_to be(0) }
it { expect(foo).to be_truthy }
it { expect(foo).not_to be_falsy }
it { expect(foo).to be_nil }
