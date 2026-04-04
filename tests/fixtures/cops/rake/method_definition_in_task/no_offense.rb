def helper
  puts 'help'
end

task :foo do
  helper
end

task :bar do
  class Foo
    def inside_class
      puts 'ok'
    end
  end
end

task :baz do
  bar = Class.new {
    def increment
    end
  }.new
end

namespace :ns do
  Module.new do
    def helper
    end
  end
end
