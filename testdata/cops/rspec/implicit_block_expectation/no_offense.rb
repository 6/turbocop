is_expected.to eq(something)
is_expected.to be_truthy
expect { boom }.to raise_error(StandardError)
expect { action }.to change { something }.to(new_value)
is_expected.to be_a(String)
expect(subject).to eq(42)
