specify do
  $stdout.puts("hi")
end

specify do
  $blah = StringIO.new
end

it 'uses output matcher' do
  expect { run }.to output("hello").to_stdout
end

# $stdout/$stderr in method definitions are NOT flagged
def capture_output
  $stdout = StringIO.new
  $stderr = StringIO.new
end

# $stdout/$stderr in before(:all) hooks are NOT flagged
before(:all) do
  $stdout = StringIO.new
end
