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

def notice_params
  return @notice_params if @notice_params
                                        ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  @notice_params = params[:data] || request.raw_post
  if @notice_params.blank?
    fail ParamsError, "Need a data params in GET or raw post data"
  end
  ^^^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  @notice_params
end

# Guard clause followed by bare raise (not a guard line)
def exception_class
  return @exception_class if @exception_class
                                            ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  raise NotImplementedError, "error response must define #exception_class"
end
