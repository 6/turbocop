# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  spec.required_ruby_version = '>= 3.0'
  spec.add_dependency 'foo'
  spec.authors = ['Author']
  # RUBY_VERSION is fine in comments
end
