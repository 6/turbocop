items[0..-1]
     ^^^^^^^ Style/SlicingWithRange: Prefer `items` over `items[0..-1]`.

items[1..-1]
     ^^^^^^^ Style/SlicingWithRange: Prefer `[1..]` over `[1..-1]`.

arr[2..-1]
   ^^^^^^^ Style/SlicingWithRange: Prefer `[2..]` over `[2..-1]`.

items[0..nil]
     ^^^^^^^^ Style/SlicingWithRange: Prefer `items` over `items[0..nil]`.

items[0...nil]
     ^^^^^^^^^ Style/SlicingWithRange: Prefer `items` over `items[0...nil]`.

items[1..nil]
     ^^^^^^^^ Style/SlicingWithRange: Prefer `[1..]` over `[1..nil]`.

items[1...nil]
     ^^^^^^^^^ Style/SlicingWithRange: Prefer `[1...]` over `[1...nil]`.

raw[idx..nil]
   ^^^^^^^^^^ Style/SlicingWithRange: Prefer `[idx..]` over `[idx..nil]`.

raw[idx...nil]
   ^^^^^^^^^^^ Style/SlicingWithRange: Prefer `[idx...]` over `[idx...nil]`.

print next_log[log.length .. - 1]
              ^^^^^^^^^^^^^^^^^^^ Style/SlicingWithRange: Prefer `[log.length..]` over `[log.length .. - 1]`.

print next_log[log.length .. - 1]
              ^^^^^^^^^^^^^^^^^^^ Style/SlicingWithRange: Prefer `[log.length..]` over `[log.length .. - 1]`.
