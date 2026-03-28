# nitrocop-filename: mutex_m.gemspec
begin
  require_relative "lib/mutex_m"
rescue LoadError
  # for Ruby core repository
  require_relative "mutex_m"
  ^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RequireRelativeSelfPath: Remove the `require_relative` that requires itself.
end
