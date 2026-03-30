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

require 'b'
# comment
require 'a'
^ Style/RequireOrder: Sort `require` in alphabetical order.

require 'b'
# require 'z'
require 'a'
^ Style/RequireOrder: Sort `require` in alphabetical order.

require 'b'
# multiple
# comments
require 'a'
^ Style/RequireOrder: Sort `require` in alphabetical order.

require("b")
require("a")
^ Style/RequireOrder: Sort `require` in alphabetical order.

require_relative("b")
require_relative("a")
^ Style/RequireOrder: Sort `require_relative` in alphabetical order.

﻿require 'webmachine/adapter'
require 'rack'
^ Style/RequireOrder: Sort `require` in alphabetical order.
require 'webmachine/constants'

require 'promise'
  
require 'facets/hash/symbolize_keys'
^ Style/RequireOrder: Sort `require` in alphabetical order.

require 'kconv' if(RUBY_VERSION.start_with? '1.9')
require 'date'
^ Style/RequireOrder: Sort `require` in alphabetical order.

require "@hotwired/stimulus", :Application
require "./controllers/counter_controller.js", :CounterController
^ Style/RequireOrder: Sort `require` in alphabetical order.
require "./controllers/accordion_controller.js", :AccordionController
^ Style/RequireOrder: Sort `require` in alphabetical order.

require 'readline'
require 'yaml'
Apricot.require "repl"
^ Style/RequireOrder: Sort `require` in alphabetical order.

require 'rake'
require 'bundler'; Bundler.setup
^ Style/RequireOrder: Sort `require` in alphabetical order.
require 'rspec/core/rake_task'

require "mocktail"; require "minitest/autorun"
                    ^ Style/RequireOrder: Sort `require` in alphabetical order.
