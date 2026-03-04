# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  spec.add_dependency 'foo', '~> 1.0'
  spec.add_dependency 'bar', '>= 2.0'
  spec.add_development_dependency 'rspec', '~> 3.0'
  spec.add_dependency %q<os>, "~> 1.1", ">= 1.1.4"
  spec.add_dependency %q(parser), '~> 3.0'
  spec.add_dependency %q[json], '>= 2.0'
  spec.add_dependency %Q<rubocop-ast>, '~> 1.0'
  spec.add_runtime_dependency %q<rake>, '~> 13.0'
  spec.add_development_dependency %q<minitest>, '~> 5.0'
  spec.authors = ['Author']
  # ENV.fetch as version arg — RuboCop sees the third arg '< 3.0' as a version string
  spec.add_dependency 'client', ENV.fetch('TEST_VERSION', '>= 1.0'), '< 3.0'
  # Variable first arg in .each block — version string in second arg still counts
  %w[core utils].each { |comp| spec.add_dependency comp, '>= 6.1.4' }
  %w[core utils].each do |gem_name|
    spec.add_dependency(gem_name, '>= 6.1')
  end
  # Version arg before if/unless modifier — version IS present
  spec.add_dependency 'filemagic', '~> 0.7' unless RUBY_ENGINE == 'jruby'
  spec.add_dependency 'rubocop', '1.50.0' unless ENV['CI']
end
