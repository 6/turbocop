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

it 'should enable debug console', skip: ENV['CI'] do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/NoExpectationExample: No expectation found in this example.
  with_context do |context|
    context.enable_debug_console!
  end
end

it 'creates a lot of transfers quickly with metadata & metadata column on lines table', skip: ActiveRecord.version.version < '5' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/NoExpectationExample: No expectation found in this example.
  DoubleEntry.config.json_metadata = true
end

it "can handle when the stream is reopened to a system stream", :skip => RSpec::Support::OS.windows? do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/NoExpectationExample: No expectation found in this example.
  send_notification :deprecation_summary, null_notification
end

it 'does support inherited matchers', :skip => options.include?(:allow_other_matchers) do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/NoExpectationExample: No expectation found in this example.
  receiver.foo
end
