expect([user1, user2, user3]).to all(be_valid)
[user1, user2, user3].each { |user| allow(user).to receive(:method) }
[user1, user2, user3].each { |_user| do_something }
items.map { |item| item.name }
users.each { |user| user.save }
expect(users).to all(be_a(User))
# Block param NOT used directly in expect() — not flagged
%w(foo bar).each do |type|
  expect(data['alerts'][type]).to eq('true')
end
