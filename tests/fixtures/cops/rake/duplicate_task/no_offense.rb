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

ADAPTERS.each do |adapter|
  namespace adapter do
    task :adapter do
      ENV['ADAPTER'] = adapter
    end
  end
end

task :adapter do
  ENV['ADAPTER'] = nil
end
