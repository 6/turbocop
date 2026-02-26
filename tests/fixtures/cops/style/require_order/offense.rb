require 'b'
require 'a'
^ Style/RequireOrder: Sort `require` in alphabetical order.

require 'z'
require 'c'
^ Style/RequireOrder: Sort `require` in alphabetical order.
require 'a'
^ Style/RequireOrder: Sort `require` in alphabetical order.

require 'd'
require 'a'
^ Style/RequireOrder: Sort `require` in alphabetical order.
require 'b'
^ Style/RequireOrder: Sort `require` in alphabetical order.
require 'c'
^ Style/RequireOrder: Sort `require` in alphabetical order.

require_relative 'z'
require_relative 'b'
^ Style/RequireOrder: Sort `require_relative` in alphabetical order.
require_relative 'c'
^ Style/RequireOrder: Sort `require_relative` in alphabetical order.
require_relative 'd'
^ Style/RequireOrder: Sort `require_relative` in alphabetical order.

require 'c'
require 'a' if foo
^ Style/RequireOrder: Sort `require` in alphabetical order.
require 'b'
^ Style/RequireOrder: Sort `require` in alphabetical order.
