# Monuple: single value wrapped in array is redundant
[single_value].hash
               ^^^^ Security/CompoundHash: Delegate hash directly without wrapping in an array when only using a single value.
[@foo].hash
       ^^^^ Security/CompoundHash: Delegate hash directly without wrapping in an array when only using a single value.
[x].hash
    ^^^^ Security/CompoundHash: Delegate hash directly without wrapping in an array when only using a single value.

# Redundant: calling .hash on ANY element of hashed array flags that element
[1, 2.hash, 3].hash
    ^^^^^^ Security/CompoundHash: Calling `.hash` on elements of a hashed array is redundant.
[@foo.hash, @bar.hash].hash
 ^^^^^^^^^ Security/CompoundHash: Calling `.hash` on elements of a hashed array is redundant.
            ^^^^^^^^^ Security/CompoundHash: Calling `.hash` on elements of a hashed array is redundant.
[a.hash, b.hash, c.hash].hash
 ^^^^^^ Security/CompoundHash: Calling `.hash` on elements of a hashed array is redundant.
         ^^^^^^ Security/CompoundHash: Calling `.hash` on elements of a hashed array is redundant.
                 ^^^^^^ Security/CompoundHash: Calling `.hash` on elements of a hashed array is redundant.

# Combinator: XOR inside def hash
def hash
  @foo.hash ^ @bar.hash
  ^^^^^^^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
end

# Combinator: XOR without sub-calls to hash
def hash
  1 ^ 2 ^ 3
  ^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
end

# Combinator: addition
def hash
  foo.hash + bar.hash
  ^^^^^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
end

# Combinator: multiplication
def hash
  to_s.hash * -1
  ^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
end

# Combinator: bitwise OR
def hash
  ([@addr, @mask_addr].hash << 1) | (ipv4? ? 0 : 1)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
end

# Combinator: def self.hash (singleton method)
def object.hash
  1.hash ^ 2.hash ^ 3.hash
  ^^^^^^^^^^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
end

# Combinator: define_method(:hash)
define_method(:hash) do
  1.hash ^ 2.hash ^ 3.hash
  ^^^^^^^^^^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
end

# Combinator: define_singleton_method(:hash)
define_singleton_method(:hash) do
  1.hash ^ 2.hash ^ 3.hash
  ^^^^^^^^^^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
end

# Combinator: XOR assignment (op-asgn)
def hash
  h = 0
  things.each do |thing|
    h ^= thing.hash
    ^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
  end
  h
end

# Combinator: addition assignment
def hash
  h = 0
  things.each do |thing|
    h += thing.hash
    ^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
  end
  h
end

# Combinator: intermediate variable
def hash
  value = 1.hash ^ 2.hash ^ 3.hash
          ^^^^^^^^^^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
  value
end

# Combinator: XOR between array hash and class
def hash
  [red, blue, green, alpha].hash ^ self.class.hash
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
end

# Combinator: index operator write (h[:key] += value) inside def hash
def hash
  h = 0
  items.each do |item|
    h ^= item.hash
    ^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
  end
  h
end

# Redundant: bare hash call (no receiver) in hashed array
[created_at, archived_at, name, id, hash, updated_at].hash
                                    ^^^^ Security/CompoundHash: Calling `.hash` on elements of a hashed array is redundant.
