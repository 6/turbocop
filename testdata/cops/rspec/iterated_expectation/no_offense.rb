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
# Multiple block parameters — not flagged (RuboCop requires exactly one)
[
  [bug_report, label_1, 'label_1'],
  [feature_request, label_2, 'label_2']
].each do |report_data, label, label_name|
  expect(report_data).to include(id: label.id, name: label_name)
end
