def foo(*args, **kwargs, &block)
       ^^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  bar(*args, **kwargs, &block)
end

def baz(*args, &block)
       ^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  qux(*args, &block)
end

def test(*args, **opts, &blk)
        ^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  other(*args, **opts, &blk)
end
