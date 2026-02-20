# turbocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.add_dependency 'aaa', '~> 1.0'
  spec.add_dependency 'bbb', '~> 2.0'
  spec.add_dependency 'ccc'

  spec.add_development_dependency 'alpha'
  spec.add_development_dependency 'beta'
end
