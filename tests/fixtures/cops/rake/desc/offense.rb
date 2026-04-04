task :foo do
^^^^ Rake/Desc: Describe the task with `desc` method.
  puts 'foo'
end

task :bar
^^^^ Rake/Desc: Describe the task with `desc` method.

task :baz do
^^^^ Rake/Desc: Describe the task with `desc` method.
  puts 'baz'
end

begin
  require 'foo'
rescue LoadError
  task :gem do
  ^^^^ Rake/Desc: Describe the task with `desc` method.
    abort 'not available'
  end
  task :package do
  ^^^^ Rake/Desc: Describe the task with `desc` method.
    abort 'not available'
  end
end
