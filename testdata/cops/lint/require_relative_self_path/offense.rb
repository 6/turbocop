# rblint-filename: foo.rb
require_relative 'foo'
^^^^^^^^^^^^^^^^^^^^^^ Lint/RequireRelativeSelfPath: Remove the `require_relative` that requires itself.
require_relative 'foo.rb'
^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RequireRelativeSelfPath: Remove the `require_relative` that requires itself.
require_relative './foo'
^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RequireRelativeSelfPath: Remove the `require_relative` that requires itself.
require_relative 'bar'
