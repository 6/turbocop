specify do
  $stdout = StringIO.new
  ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stdout` instead of mutating $stdout.
end

before(:each) do
  $stderr = StringIO.new
  ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stderr` instead of mutating $stderr.
end

it 'captures output' do
  $stdout = StringIO.new
  ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stdout` instead of mutating $stdout.
end
