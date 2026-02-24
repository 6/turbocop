def foo(&block)
        ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar(&block)
end
def baz(&block)
        ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  yield_with(&block)
end
def qux(&block)
        ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  other(&block)
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
end
