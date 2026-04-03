desc 'Do foo'
task :foo do
  puts 'foo'
end

desc 'Do bar'
task :bar

task :default

task default: [:foo]
