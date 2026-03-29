require 'json'
require 'yaml'
require 'net/http'
require_relative 'foo'
require_relative 'bar'
require 'foo'

# Same require in different methods is OK (different scope)
def setup
  require 'json'
end

def teardown
  require 'json'
end

# Same require in conditional branches is OK (different scope)
if RUBY_VERSION >= '3.0'
  require 'fiber'
else
  require 'fiber'
end

# Same require in a class vs top-level is OK
class MyApp
  require 'json'
end

# Wrapped requires have different parents — not duplicates
assert require('test_helper')
result = require 'test_helper'

# Block-attached require has a block parent, so a later plain require is independent
require('block_feature'){ 1 }
require 'block_feature'

# Multi-statement block body gets its own begin/statements parent
require('block_nested') do
  1
  Kernel.require 'block_nested'
end
