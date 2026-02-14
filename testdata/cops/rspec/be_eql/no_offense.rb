it { expect(foo).to eql('foo') }
it { expect(foo).to_not eql(1) }
it { expect(foo).not_to eql(1) }
it { expect(foo).to eq(1) }
it { expect(foo).to be(true) }
