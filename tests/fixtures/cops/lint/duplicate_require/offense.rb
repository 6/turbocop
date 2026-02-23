require 'json'
require 'yaml'
require 'json'
^^^^^^^^^^^^^^ Lint/DuplicateRequire: Duplicate `require` detected.

require_relative 'foo'
require_relative 'bar'
require_relative 'foo'
^^^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateRequire: Duplicate `require` detected.

require 'net/http'
require 'net/http'
^^^^^^^^^^^^^^^^^^ Lint/DuplicateRequire: Duplicate `require` detected.
