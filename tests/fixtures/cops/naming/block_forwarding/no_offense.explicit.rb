# explicit block forwarding — already using named block
def foo(&block)
  bar(&block)
end
# explicit block forwarding without body
def empty_explicit(&block)
end
# no block param at all
def no_block(arg1, arg2)
end
# block used as variable
def uses_block(&block)
  block.call
end
# no arguments method
def no_args
end
