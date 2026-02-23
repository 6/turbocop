def +(other)
end
def -(other)
end
def ==(other)
end
def <=>(other)
end
def [](index)
end

# << is excluded from this cop
def <<(callable)
end

# Singleton methods are not checked
def ANY.==(_)
  true
end

# _other is accepted
def +(_other)
end
