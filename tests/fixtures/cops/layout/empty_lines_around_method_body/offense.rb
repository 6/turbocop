def foo

^ Layout/EmptyLinesAroundMethodBody: Extra empty line detected at method body beginning.
  bar

^ Layout/EmptyLinesAroundMethodBody: Extra empty line detected at method body end.
end
def baz

^ Layout/EmptyLinesAroundMethodBody: Extra empty line detected at method body beginning.
  qux
end
def corge
  grault

^ Layout/EmptyLinesAroundMethodBody: Extra empty line detected at method body end.
end
def some_method(
  arg
)

^ Layout/EmptyLinesAroundMethodBody: Extra empty line detected at method body beginning.
  do_something
end
def compute(value,
  factor) =

^ Layout/EmptyLinesAroundMethodBody: Extra empty line detected at method body beginning.
  value * factor
def fetch uri, method = :get, headers = {}, params = [],
          referer = current_page, redirects = 0

^ Layout/EmptyLinesAroundMethodBody: Extra empty line detected at method body beginning.
  request = build_request(uri, method, headers, params)
end
