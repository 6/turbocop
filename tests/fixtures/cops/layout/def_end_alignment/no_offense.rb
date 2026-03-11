def foo
  42
end

def bar = 42

def baz(x)
  x + 1
end

# Single-line defs should not be flagged
def qux; 42; end
def quux(x); x + 1; end

# Modifier before def: end aligns with line start, not def keyword
private_class_method def self.helper(x)
  x + 1
end

# Tab-indented defs should not be flagged when end aligns with def
	def tab_method
		42
	end

	private def tab_modifier_method
		43
	end
