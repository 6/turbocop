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

  # Bacon-style .should with a receiver is NOT an expectation (requires receiver-less call)
  it 'uses bacon style should' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/NoExpectationExample: No expectation found in this example.
    something.should.be.nil
    result.should_not.be.empty
  end
end
