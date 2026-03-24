Gem::Specification.new do |spec|
  spec.add_dependency 'aaa'
  spec.add_dependency 'bar', '~> 2.0'
  spec.add_dependency 'foo', '~> 1.0'

  spec.add_development_dependency 'alpha'
  spec.add_development_dependency 'zebra'

  s.add_runtime_dependency(%q<activesupport>, ["~> 4.2"])
  s.add_runtime_dependency(%q<tilt>, ["~> 1.4"])

  s.add_dependency(%Q<alpha>)
  s.add_dependency(%Q<zebra>)

  s.add_dependency(%q(aaa))
  s.add_dependency(%q(zoo))
end
