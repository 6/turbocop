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
