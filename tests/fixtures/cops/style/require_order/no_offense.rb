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
