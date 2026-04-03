desc 'Do foo'
task :foo do
  puts 'foo'
end

desc 'Do bar'
task :bar do
  puts 'bar'
end

namespace :ns do
  desc 'Namespaced foo'
  task :foo do
    puts 'ns:foo'
  end
end
