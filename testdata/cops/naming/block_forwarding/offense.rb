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
