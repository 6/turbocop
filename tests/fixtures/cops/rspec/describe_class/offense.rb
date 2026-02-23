describe "bad describe" do
         ^^^^^^^^^^^^^^ RSpec/DescribeClass: The first argument to describe should be the class or module being tested.
end

describe "bad describe", "blah blah" do
         ^^^^^^^^^^^^^^ RSpec/DescribeClass: The first argument to describe should be the class or module being tested.
end

describe 'foo bar', foo: :bar do
         ^^^^^^^^^ RSpec/DescribeClass: The first argument to describe should be the class or module being tested.
end
