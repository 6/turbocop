create_list :user, 3
create_list(:user, 5, :trait)
1.times { create :user }
3.times { |n| create :user, position: n }
3.times { do_something }
3.times {}
3.times { |n| create :user, repositories_count: rand }
