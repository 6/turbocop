# rblint-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  spec.add_dependency 'foo', '~> 1.0'
  spec.add_dependency 'bar', '>= 2.0'
  spec.add_development_dependency 'rspec', '~> 3.0'
  spec.authors = ['Author']
end
