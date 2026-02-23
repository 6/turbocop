# turbocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.add_dependency 'foo', '~> 1.0'
  spec.add_dependency 'bar', '~> 2.0'
       ^^^^^^^^^^^^^^ Gemspec/OrderedDependencies: Dependencies should be sorted in an alphabetical order within their section of the gemspec. Dependency `bar` should appear before `foo`.
  spec.add_dependency 'aaa'
       ^^^^^^^^^^^^^^ Gemspec/OrderedDependencies: Dependencies should be sorted in an alphabetical order within their section of the gemspec. Dependency `aaa` should appear before `bar`.

  spec.add_development_dependency 'zebra'
  spec.add_development_dependency 'alpha'
       ^^^^^^^^^^^^^^^^^^^^^^^^^^ Gemspec/OrderedDependencies: Dependencies should be sorted in an alphabetical order within their section of the gemspec. Dependency `alpha` should appear before `zebra`.
end
