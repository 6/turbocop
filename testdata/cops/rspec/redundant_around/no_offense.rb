around do |example|
  example.run
  foo
end
around do |example|
  foo { example.run }
end
around do |example|
  Timecop.freeze { example.run }
end
around do |example|
  example.run
  cleanup
end
