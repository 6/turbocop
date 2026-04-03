task :foo do
  class Foo
  ^^^^^^^^^ Rake/ClassDefinitionInTask: Do not define a class in rake task, because it will be defined to the top level.
  end
end

task :bar do
  module Bar
  ^^^^^^^^^^ Rake/ClassDefinitionInTask: Do not define a module in rake task, because it will be defined to the top level.
  end
end

namespace :ns do
  class Baz
  ^^^^^^^^^ Rake/ClassDefinitionInTask: Do not define a class in rake task, because it will be defined to the top level.
  end
end
