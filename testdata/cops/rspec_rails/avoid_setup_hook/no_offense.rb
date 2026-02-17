before do
  allow(foo).to receive(:bar)
end

navigation.setup do
  direction 'to infinity!'
end

before(:each) do
  create_users
end
