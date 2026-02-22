def foo(*args, **kwargs, &block)
        ^^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  bar(*args, **kwargs, &block)
      ^^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

def baz(*args, &block)
        ^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  qux(*args, &block)
      ^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

def test(*args, **opts, &blk)
         ^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  other(*args, **opts, &blk)
        ^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

def forward_to_super(*args, &block)
                     ^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  super(*args, &block)
        ^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

def forward_triple_to_super(*args, **opts, &block)
                            ^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  super(*args, **opts, &block)
        ^^^^^^^^^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

def method_missing(*args, &block)
                   ^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  target.send(*args, &block)
              ^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

def select(*args, &block)
           ^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  return to_enum(:select) unless block_given?
  dup.tap { |hash| hash.select!(*args, &block) }
                                ^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end

def use(*args, &block)
        ^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
  Delegator.target.use(*args, &block)
                       ^^^^^^^^^^^^^^ Style/ArgumentsForwarding: Use shorthand syntax `...` for arguments forwarding.
end
