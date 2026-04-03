desc 'Do foo'
task :foo do
  puts 'foo'
end

desc 'Do foo again'
task :foo do
^^^^ Rake/DuplicateTask: Task `foo` is defined at both test.rb (line 2) and test.rb (line 7).
  puts 'foo again'
end

desc 'Do bar'
task :bar

desc 'Do bar again'
task :bar
^^^^ Rake/DuplicateTask: Task `bar` is defined at both test.rb (line 12) and test.rb (line 15).

namespace :ns do
  desc 'Do qux'
  task :qux
end

namespace :ns do
  desc 'Do qux again'
  task :qux
  ^^^^ Rake/DuplicateTask: Task `ns:qux` is defined at both test.rb (line 19) and test.rb (line 24).
end
