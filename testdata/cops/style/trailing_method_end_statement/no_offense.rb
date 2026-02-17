def some_method
  do_stuff
end

def do_this(x)
  baz.map { |b| b.this(x) }
end

def foo
  block do
    bar
  end
end

def single_line; end

# Method with rescue — end is on its own line
def create
  do_something
rescue StandardError
  nil
end

# Method with rescue and else clause
def batch
  form.save
rescue ActionController::ParameterMissing
  flash[:alert] = "error"
else
  redirect_to root_path
end

# Method with ensure
def safe_write
  write_data
ensure
  close_file
end
