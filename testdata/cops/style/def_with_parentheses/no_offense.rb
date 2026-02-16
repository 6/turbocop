def func
end

def bar(x)
  x
end

def baz(a, b)
  a + b
end

def Test.func
  something
end

# Single-line non-endless methods require parens for syntax validity
def index() head :ok end
def nothing() head :ok end
def hello_xml_world() render template: "test/hello"; end
