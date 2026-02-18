expect(foo).not_to be_valid
expect(foo).to be_valid
expect(bar).not_to be_valid
expect(foo).to be_invalid.and be_odd
expect(foo).to be_invalid.or be_even
