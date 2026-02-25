create_list :user, 3
create_list(:user, 5, :trait)
1.times { create :user }
3.times { |n| create :user, position: n }
3.times { do_something }
3.times {}
3.times { |n| create :user, repositories_count: rand }
# Value omission args should not be flagged
3.times { create(:item, checklist:, checked: true) }
2.times { create(:refund, purchase:, amount_cents: 10) }
# Array.new pattern is NOT flagged by RuboCop
Array.new(3) { create(:user) }
# Array with different create calls
[create(:user), create(:user, age: 18)]
# Array with single create
[create(:user)]
# Empty array
[]
