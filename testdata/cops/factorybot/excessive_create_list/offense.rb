create_list(:merge_requests, 11)
                             ^^ FactoryBot/ExcessiveCreateList: Avoid using `create_list` with more than 10 items.
FactoryBot.create_list(:merge_requests, 11)
                                        ^^ FactoryBot/ExcessiveCreateList: Avoid using `create_list` with more than 10 items.
FactoryBot.create_list('warehouse/user', 11)
                                         ^^ FactoryBot/ExcessiveCreateList: Avoid using `create_list` with more than 10 items.
create_list(:merge_requests, 1000, state: :opened)
                             ^^^^ FactoryBot/ExcessiveCreateList: Avoid using `create_list` with more than 10 items.
