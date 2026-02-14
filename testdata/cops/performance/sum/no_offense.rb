[1, 2, 3].sum
[1, 2, 3].inject(0) { |s, v| s + v }
[1, 2, 3].inject(:*)
[1, 2, 3].reduce(1, :*)
arr.sum { |x| x.value }
