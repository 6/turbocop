x = { a: 1,
  b: 2,
  ^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.
  c: 3 }
  ^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.
y = { d: 4,
        e: 5 }
        ^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.
z = { f: 6,
    g: 7 }
    ^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.

# Separator/value alignment: extra spaces after colon (key style)
hash1 = {
  a:   0,
  ^^^^^^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.
  bb:1,
  ^^^^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.
}

# Separator/value alignment: hash rockets with extra spaces
hash2 = {
  'ccc'=> 2,
  ^^^^^^^^^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.
  'dddd' =>  3
  ^^^^^^^^^^^^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.
}

# First pair with bad spacing (even first pair gets checked for separator/value)
hash3 = {
  :a   => 0,
  ^^^^^^^^^^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.
  :bb => 1,
}

# Mixed offenses: key misalignment AND separator/value spacing
hash4 = { :a   => 0,
          ^^^^^^^^^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.
    :bb  => 1,
    ^^^^^^^^^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.
              :ccc  =>2 }
              ^^^^^^^^^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.

# Keyword splat alignment
{foo: 'bar',
       **extra
       ^^^^^^^ Layout/HashAlignment: Align keyword splats with the rest of the hash if it spans more than one line.
}
