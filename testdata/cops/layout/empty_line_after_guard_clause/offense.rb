def foo
  return if need_return?
                       ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  bar
end

def baz
  raise "error" unless valid?
                            ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  do_work
end

def quux
  return unless something?
                         ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  process
end
