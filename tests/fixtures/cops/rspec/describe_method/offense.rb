describe Some::Class, 'nope' do
                      ^^^^^^ RSpec/DescribeMethod: The second argument to describe should be the method being tested. '#instance' or '.class'.
end

describe MyClass, 'incorrect_usage' do
                  ^^^^^^^^^^^^^^^^^ RSpec/DescribeMethod: The second argument to describe should be the method being tested. '#instance' or '.class'.
end

describe AnotherClass, 'something else' do
                       ^^^^^^^^^^^^^^^^ RSpec/DescribeMethod: The second argument to describe should be the method being tested. '#instance' or '.class'.
end

# String first argument (not a constant) — RuboCop still checks the second arg
RSpec.describe "Boards", "Creating a view from a Global Context" do
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/DescribeMethod: The second argument to describe should be the method being tested. '#instance' or '.class'.
end

RSpec.describe "Calendars", "index" do
                            ^^^^^^^ RSpec/DescribeMethod: The second argument to describe should be the method being tested. '#instance' or '.class'.
end

# Method call first argument — still checks second arg
describe Puppet::Type.type(:package), "when choosing a default provider" do
                                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/DescribeMethod: The second argument to describe should be the method being tested. '#instance' or '.class'.
end
