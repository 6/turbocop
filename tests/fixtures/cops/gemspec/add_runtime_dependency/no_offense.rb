# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  spec.add_dependency 'foo', '~> 1.0'
  spec.add_dependency 'bar'
  spec.add_development_dependency 'rspec'
  spec.authors = ['Author']
  # spec.add_runtime_dependency 'foo'
  # Old dependency: spec.add_runtime_dependency 'bar', '~> 1.0'
end
