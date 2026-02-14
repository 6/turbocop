specify do
  $stdout.puts("hi")
end

specify do
  $blah = StringIO.new
end

it 'uses output matcher' do
  expect { run }.to output("hello").to_stdout
end
