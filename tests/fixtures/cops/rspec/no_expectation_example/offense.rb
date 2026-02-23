RSpec.describe Foo do
  it { bar }
  ^^^^^^^^^^ RSpec/NoExpectationExample: No expectation found in this example.

  specify { baz }
  ^^^^^^^^^^^^^^^ RSpec/NoExpectationExample: No expectation found in this example.

  it 'does nothing useful' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/NoExpectationExample: No expectation found in this example.
    x = 1
    y = 2
    z = x + y
  end
end
