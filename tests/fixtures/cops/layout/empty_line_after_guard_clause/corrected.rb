def foo
  return if need_return?

  bar
end

def baz
  raise "error" unless valid?

  do_work
end

def quux
  return unless something?

  process
end

def notice_params
  return @notice_params if @notice_params

  @notice_params = params[:data] || request.raw_post
  if @notice_params.blank?
    fail ParamsError, "Need a data params in GET or raw post data"
  end

  @notice_params
end

# Guard clause followed by bare raise (not a guard line)
def exception_class
  return @exception_class if @exception_class

  raise NotImplementedError, "error response must define #exception_class"
end
