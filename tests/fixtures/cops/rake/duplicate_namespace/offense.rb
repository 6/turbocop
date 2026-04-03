namespace :foo do
  desc 'Do bar'
  task :bar
end

namespace :foo do
^^^^^^^^^ Rake/DuplicateNamespace: Namespace `foo` is defined at both test.rb (line 1) and test.rb (line 6).
  desc 'Do baz'
  task :baz
end

namespace :qux do
  desc 'Do a'
  task :a
end

namespace :qux do
^^^^^^^^^ Rake/DuplicateNamespace: Namespace `qux` is defined at both test.rb (line 11) and test.rb (line 16).
  desc 'Do b'
  task :b
end

namespace :third do
  desc 'Do x'
  task :x
end

namespace :third do
^^^^^^^^^ Rake/DuplicateNamespace: Namespace `third` is defined at both test.rb (line 21) and test.rb (line 26).
  desc 'Do y'
  task :y
end
