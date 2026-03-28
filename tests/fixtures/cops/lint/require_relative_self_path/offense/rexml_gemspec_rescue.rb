# nitrocop-filename: rexml.gemspec
begin
  require_relative "lib/rexml/rexml"
rescue LoadError
  # for Ruby core repository
  require_relative "rexml"
  ^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RequireRelativeSelfPath: Remove the `require_relative` that requires itself.
end
