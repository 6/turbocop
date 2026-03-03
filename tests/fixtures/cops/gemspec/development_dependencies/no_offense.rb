# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  spec.add_dependency 'foo', '~> 1.0'
  spec.add_dependency 'bar'
  spec.authors = ['Author']
  spec.summary = 'An example gem'

  # Dynamic (non-string-literal) arguments should not trigger offense
  common_gemspec.development_dependencies.each do |dep|
    spec.add_development_dependency dep.name, *dep.requirement.as_list
  end

  deps.each { |d| spec.add_development_dependency(d) }
end
