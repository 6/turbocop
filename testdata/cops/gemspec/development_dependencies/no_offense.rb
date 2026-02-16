# rblint-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  spec.add_dependency 'foo', '~> 1.0'
  spec.add_dependency 'bar'
  spec.authors = ['Author']
  spec.summary = 'An example gem'
end
