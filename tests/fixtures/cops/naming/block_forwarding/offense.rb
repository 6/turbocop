def foo(&block)
        ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar(&block)
      ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
def baz(&block)
        ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  yield_with(&block)
             ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
def qux(&block)
        ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  other(&block)
        ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# yield is forwarding
def with_yield(&block)
               ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  yield
end
# unused block param (no body)
def empty_body(&block)
               ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# unused block param (body exists but doesn't reference block)
def unused_param(&block)
                 ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  something_else
end
# symbol proc in body (block unused)
def with_symbol_proc(&block)
                     ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar(&:do_something)
end
# forwarding in singleton method
def self.singleton_fwd(&block)
                       ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar(&block)
      ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# multiple forwarding usages
def multi_forward(&block)
                  ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar(&block)
      ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  baz(qux, &block)
           ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# forwarding without parentheses
def no_parens arg, &block
                   ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar &block
      ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  baz qux, &block
           ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# forwarding with other proc arguments
def other_procs(bar, &block)
                     ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  delegator.foo(&bar).each(&block)
                           ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# space between & and param name — only param is flagged, not body forwarding
# (body uses "&block" without space, so source texts don't match)
def transmit uri, req, payload, & block
                                ^^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  process_result(res, start_time, tempfile, &block)
end
# space between & and param name AND matching space in body forwarding —
# both param and body are flagged because source texts match
def forward_with_spaces(& block)
                        ^^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  call_something(& block)
                 ^^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
def multi_forward_spaces(& block)
                         ^^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  foo(& block)
      ^^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar(& block)
      ^^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# forwarding in nested def shares same block param name — counted as body forwarding
# for the outer method (RuboCop traverses into nested defs).
# Inner def has keyword param so it gets skipped by the keyword check.
def outer_method(&block)
                 ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  Class.new do
    def initialize(*args, key: true, &block)
      super(*args, &block)
                   ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
    end
  end
end
# space in param — block used as local variable, but RuboCop's source text
# comparison fails to detect lvar usage, so param offense still fires
def transform(& block)
              ^^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  items = [] unless defined?(@items)
  items << block if block_given?
  items
end
