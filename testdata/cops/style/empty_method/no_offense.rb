def foo; end

def bar
  42
end

def baz = 42

def self.foo; end

def multi
  bar
end

def self.multi
  bar
end

def single_line_body; 42; end
