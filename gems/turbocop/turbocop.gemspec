# frozen_string_literal: true

require_relative "lib/turbocop"

Gem::Specification.new do |spec|
  spec.name     = "turbocop"
  spec.version  = Turbocop::VERSION
  spec.authors  = ["6"]

  spec.summary     = "Fast Ruby linter targeting RuboCop compatibility"
  spec.description = "A Ruby linter written in Rust that reads your existing .rubocop.yml " \
                     "and runs 900+ cops."
  spec.homepage    = "https://github.com/6/turbocop"
  spec.license     = "MIT"

  spec.required_ruby_version = ">= 3.1.0"

  spec.metadata["source_code_uri"] = spec.homepage
  spec.metadata["changelog_uri"]   = "#{spec.homepage}/releases"

  spec.files       = Dir["lib/**/*", "exe/**/*", "libexec/**/*"]
  spec.bindir      = "exe"
  spec.executables = ["turbocop"]
end
