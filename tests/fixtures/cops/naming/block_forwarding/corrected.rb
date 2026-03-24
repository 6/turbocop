def foo(&)
  bar(&)
end
def baz(&)
  yield_with(&)
end
def qux(&)
  other(&)
end
# yield is forwarding
def with_yield(&)
  yield
end
# unused block param (no body)
def empty_body(&)
end
# unused block param (body exists but doesn't reference block)
def unused_param(&)
  something_else
end
# symbol proc in body (block unused)
def with_symbol_proc(&)
  bar(&:do_something)
end
# forwarding in singleton method
def self.singleton_fwd(&)
  bar(&)
end
# multiple forwarding usages
def multi_forward(&)
  bar(&)
  baz(qux, &)
end
# forwarding without parentheses
def no_parens arg, &
  bar &
  baz qux, &
end
# forwarding with other proc arguments
def other_procs(bar, &)
  delegator.foo(&bar).each(&)
end
# space between & and param name — only param is flagged, not body forwarding
# (body uses "&block" without space, so source texts don't match)
def transmit uri, req, payload, &
  process_result(res, start_time, tempfile, &block)
end
# space between & and param name AND matching space in body forwarding —
# both param and body are flagged because source texts match
def forward_with_spaces(&)
  call_something(&)
end
def multi_forward_spaces(&)
  foo(&)
  bar(&)
end
# forwarding in nested def shares same block param name — counted as body forwarding
# for the outer method (RuboCop traverses into nested defs).
# Inner def has keyword param so it gets skipped by the keyword check.
def outer_method(&)
  Class.new do
    def initialize(*args, key: true, &block)
      super(*args, &)
    end
  end
end
# space in param — block used as local variable, but RuboCop's source text
# comparison fails to detect lvar usage, so param offense still fires
def transform(&)
  items = [] unless defined?(@items)
  items << block if block_given?
  items
end
