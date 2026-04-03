task :foo do
  def helper
  ^^^^^^^^^^ Rake/MethodDefinitionInTask: Do not define a method in rake task, because it will be defined to the top level.
    puts 'help'
  end
end

namespace :ns do
  def another_helper
  ^^^^^^^^^^^^^^^^^^ Rake/MethodDefinitionInTask: Do not define a method in rake task, because it will be defined to the top level.
    puts 'help'
  end
end

task :baz do
  def yet_another
  ^^^^^^^^^^^^^^^ Rake/MethodDefinitionInTask: Do not define a method in rake task, because it will be defined to the top level.
  end
end
