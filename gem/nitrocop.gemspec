# frozen_string_literal: true

require_relative "lib/nitrocop"

Gem::Specification.new do |spec|
  spec.name     = "nitrocop"
  spec.version  = Nitrocop::VERSION
  spec.authors  = ["6"]

  spec.summary     = "Fast Ruby linter targeting RuboCop compatibility"
  spec.description = "A Ruby linter written in Rust that reads your existing .rubocop.yml " \
                     "and runs 900+ cops."
  spec.homepage    = "https://github.com/6/nitrocop"
  spec.license     = "MIT"

  spec.required_ruby_version = ">= 3.1.0"

  spec.metadata["source_code_uri"] = spec.homepage
  spec.metadata["changelog_uri"]   = "#{spec.homepage}/releases"

  spec.files       = Dir["lib/**/*", "exe/**/*", "libexec/**/*"]
  spec.bindir      = "exe"
  spec.executables = ["nitrocop"]
end
