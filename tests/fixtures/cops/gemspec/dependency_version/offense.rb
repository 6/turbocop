# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.add_dependency 'foo'
       ^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_runtime_dependency 'bar'
       ^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_development_dependency 'baz'
       ^^^^^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_dependency %q<os>
       ^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_runtime_dependency %q(parser)
       ^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_development_dependency %q[minitest]
       ^^^^^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_dependency 'interp', "~> #{VERSION}"
       ^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  # != is not a version operator per RuboCop's regex /^\s*[~<>=]*\s*[0-9.]+/
  spec.add_dependency 'excluded', '!= 0.3.1'
       ^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  # Array-wrapped version strings don't count — RuboCop only matches direct str args
  spec.add_dependency(%q<json_pure>.freeze, [">= 0"])
       ^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_dependency(%q<coffee-script>, ["~> 2.4.1"])
       ^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  spec.add_dependency 'multi-ver', [">= 1.0", "< 3.0"]
       ^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  # Version inside ENV.fetch() is not a direct str arg
  spec.add_dependency 'model', ENV.fetch('RAILS_VER', '>= 4.0.0')
       ^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  # Version inside parenthesized ternary is not a direct str arg
  spec.add_development_dependency "parser", (RUBY_VERSION < '2.3' ? '< 2.0.0' : '> 2.0.0')
       ^^^^^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
  # Version in if/unless modifier condition is not a version arg
  spec.add_development_dependency 'coverage' if RUBY_VERSION >= '2.7.0'
       ^^^^^^^^^^^^^^^^^^^^^^^^^^ Gemspec/DependencyVersion: Dependency version is required.
end
