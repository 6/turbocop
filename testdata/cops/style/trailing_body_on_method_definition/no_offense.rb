def some_method
  do_stuff
end

def f(x)
  b = foo
  b[c: x]
end

def endless_method = do_stuff

def single_line; end

# Method with rescue (implicit begin) — body is NOT on def line
def create
  report = ErrorReport.new
rescue StandardError
  nil
end

# Single-line method with body — not multiline so not flagged
def compact; do_stuff; end

# Method with ensure
def safe_write
  write_data
ensure
  close_file
end
