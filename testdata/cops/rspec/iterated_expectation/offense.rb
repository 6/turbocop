[user1, user2, user3].each { |user| expect(user).to be_valid }
^^^^^^^^^^^^^^^^^^^^^ RSpec/IteratedExpectation: Prefer using the `all` matcher instead of iterating over an array.
[item1, item2].each { |item| expect(item).to be_a(Item) }
^^^^^^^^^^^^^^ RSpec/IteratedExpectation: Prefer using the `all` matcher instead of iterating over an array.
users.each do |user|
^^^^^ RSpec/IteratedExpectation: Prefer using the `all` matcher instead of iterating over an array.
  expect(user).to be_valid
end
