x = { a: 1,
      b: 2,
      c: 3 }
y = { d: 4 }
{ a: 1, b: 2 }
z = {
  e: 5,
  f: 6
}

# Element after closing brace (not first on its line)
render json: {
  redirect_to: path,
}, status: 200

# Correctly spaced hash rockets (key style)
hash1 = {
  :a => 0,
  :bb => 1
}
hash2 = {
  'a' => 0,
  'bb' => 1
}

# Correctly spaced colons (key style)
hash3 = {
  aa: 0,
  b: 1,
}

# Several pairs per line
func(a: 1, bb: 2,
     ccc: 3, dddd: 4)

# Pairs that don't start a line
render :json => {:a => messages,
                 :b => :json}, :status => 404

# Value on new line (ok)
hash4 = {
  'a' =>
    0,
  'bbb' => 1
}

# Keyword splats aligned correctly
{foo: 'bar',
 **extra}

# Hash value omission
func(a:,
     b:)
