task foo: :environment do
  puts "hello"
end
task :bar => :environment do
  puts "world"
end
