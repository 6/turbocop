expect(true).to be_truthy
create_list(:merge_requests, value)
create_list(:merge_requests, 9)
create_list(:merge_requests, 10)
create_list(:merge_requests, 5, state: :opened)
build_list(:merge_requests, 20)
