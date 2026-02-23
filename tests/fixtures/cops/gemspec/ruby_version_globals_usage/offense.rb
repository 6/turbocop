# turbocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.required_ruby_version = ">= #{RUBY_VERSION}"
                                     ^^^^^^^^^^^^ Gemspec/RubyVersionGlobalsUsage: Do not use `RUBY_VERSION` in gemspec.
  spec.summary = "Supports Ruby #{RUBY_VERSION}"
                                  ^^^^^^^^^^^^ Gemspec/RubyVersionGlobalsUsage: Do not use `RUBY_VERSION` in gemspec.
  if RUBY_VERSION >= '3.0'
     ^^^^^^^^^^^^ Gemspec/RubyVersionGlobalsUsage: Do not use `RUBY_VERSION` in gemspec.
    spec.add_dependency 'modern_gem'
  end
end
