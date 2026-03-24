task foo: :environment do
  puts "hello"
end
task :bar => :environment do
  puts "world"
end
task :notices_delete, [:problem_id] => [:environment] do
  puts "delete"
end
task :baz, [:arg] => [:environment, :other] do
  puts "multi deps"
end
task :default do
  puts "default task"
end
task default: [:test] do
  puts "default with deps"
end
task setup: :database do
  puts "setup depends on database"
end
task foo: dep do
  puts "method call dependency"
end
task foo: [dep, :bar] do
  puts "method call in array dependency"
end
task :foo, [:arg] => dep do
  puts "args with method call dependency"
end
task :generate => [:environment] do
  puts "hash rocket with array"
end
task name => new_name do
  puts "local variable with hash rocket dependency"
end
task({name => dependencies}, &block)
task({name => dependencies}) { puts "explicit hash with deps" }
