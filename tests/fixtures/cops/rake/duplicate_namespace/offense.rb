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

namespace ""
^ Rake/DuplicateNamespace: Namespace `foo` is defined at both repos/Shopify__ruby-lsp__0d5d95f/test/fixtures/rake.rake:1 and repos/Shopify__ruby-lsp__0d5d95f/test/fixtures/rake.rake:8.

contents.gsub!(/::#{namespace}/, "::#{prefix}::#{namespace}")
^ Rake/DuplicateNamespace: Namespace `` is defined at both repos/gel-rb__gel__34b69dc/tasks/automatiek.rake:108 and repos/gel-rb__gel__34b69dc/tasks/automatiek.rake:108.

contents.gsub!(/(?<!\w|def |:)#{namespace}\b/, "#{prefix}::#{namespace}")
^ Rake/DuplicateNamespace: Namespace `` is defined at both repos/gel-rb__gel__34b69dc/tasks/automatiek.rake:108 and repos/gel-rb__gel__34b69dc/tasks/automatiek.rake:109.

emit_indented 2, "describe \"#{namespace}::#{obj_name}\" do" unless @class_stack.any?
^ Rake/DuplicateNamespace: Namespace `` is defined at both repos/mvz__gir_ffi__281f517/tasks/test.rake:91 and repos/mvz__gir_ffi__281f517/tasks/test.rake:109.
