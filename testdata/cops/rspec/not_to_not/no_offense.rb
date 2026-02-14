it { expect(false).not_to be_true }
it { expect(nil).not_to be_nil }
it { expect(0).not_to eq(1) }
it { expect(foo).to be_truthy }
it { expect(bar).to eq(1) }
