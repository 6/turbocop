before do
  allow(Foo).to receive(:foo).and_return(baz)
  allow(Bar).to receive(:bar).and_return(bar)
  allow(Baz).to receive(:baz).and_return(foo)
end
before do
  allow(Service).to receive(:foo) { baz }
  allow(Service).to receive(:bar) { bar }
end
# String args to receive() should not trigger receive_messages
before(:each) do
  allow(self).to receive('action_name').and_return(action_name)
  allow(self).to receive('current_page?').and_return(false)
end

# Heredoc return values are excluded from receive_messages aggregation.
before do
  allow(provider).to receive(:different?).and_return(true)
  allow(provider).to receive(:read_crontab).and_return(<<~CRONTAB)
    0 2 * * * /some/command
  CRONTAB
end

# Multi-argument and_return calls are excluded.
before do
  allow(s3_object).to receive(:content_length).and_return(100, 105)
  allow(s3_object).to receive(:presigned_url).and_return(path_one, path_two)
end

# Chains after and_return are excluded.
before do
  allow(service).to receive(:foo).and_return(1).ordered
  allow(service).to receive(:bar).and_return(2)
end

# Same receive arg on same object (all same message) - no offense
before do
  allow(Foo).to receive(:foo).and_return(bar)
  allow(Foo).to receive(:foo).and_return(baz)
  allow(Foo).to receive(:bar).and_return(qux)
end

# Splat return values are excluded.
before do
  allow(Service).to receive(:foo).and_return(*array)
  allow(Service).to receive(:bar).and_return(*array)
end

# .with method chains are excluded (not simple stubs).
before do
  allow(Service).to receive(:foo).with(1).and_return(baz)
  allow(Service).to receive(:bar).with(2).and_return(bar)
end

# Using .and_call_original instead of .and_return
before do
  allow(Service).to receive(:foo).and_call_original
  allow(Service).to receive(:bar).and_return(qux)
  allow(Service).to receive(:baz).and_call_original
end
