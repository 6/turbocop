describe Some::Class, 'nope' do
                      ^^^^^^ RSpec/DescribeMethod: The second argument to describe should be the method being tested. '#instance' or '.class'.
end

describe MyClass, 'incorrect_usage' do
                  ^^^^^^^^^^^^^^^^^ RSpec/DescribeMethod: The second argument to describe should be the method being tested. '#instance' or '.class'.
end

describe AnotherClass, 'something else' do
                       ^^^^^^^^^^^^^^^^ RSpec/DescribeMethod: The second argument to describe should be the method being tested. '#instance' or '.class'.
end
