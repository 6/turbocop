require 'a'
require 'b'
require 'c'

require_relative 'bar'
require_relative 'foo'

x = 1
y = 2
require 'b'
require_relative 'a'
require 'e'
Bundler.require(:default)
require 'c'

require 'a'
require 'b'
require 'c' if foo

require "#{base_dir}/foo"
require "#{base_dir}/bar"

require "#{Dir.pwd}/support/a"
require "#{Dir.pwd}/support/b"

require "test/#{helper}"
require "app/#{model}"

require("a")
require("b")

require("alpha")
require "beta"

=begin test
require 'test/unit'
require 'rubygems'
require 'qualitysmith_extensions/object/ignore_access'
=end

=begin
require 'z'
require 'a'
=end

# Backslash line continuation — single require, not two separate lines
require "dependabot/bundler/update_checker/latest_version_finder/" \
  "dependency_source"
require "dependabot/bundler/update_checker/force_updater"

# require inside %() string literal — not a real require
x = %(
  require 'rexml/document'
  require 'not_valid'
)

# require inside %{} string literal — not a real require
prelude %{
  require 'rails'
  require 'action_view'
}

# rescue modifier — acts as group separator (RuboCop skips rescue_modifier nodes)
require "rubygems" rescue nil
require 'minitest/autorun'
require "rr"

# __END__ — data section, not code
require 'a'
require 'b'
__END__
require 'z'
require 'a'
