[1, 2, 3].index_with { |el| foo(el) }
[1, 2, 3].map { |el| [el, el] }.to_h
[1, 2, 3].each_with_object({}) { |el, h| h[el] = el }
{}.merge(a: 1)
[1, 2, 3].to_h { |el| [el, el] }
[1, 2, 3].index_with(&:to_s)
