"text"
:sym
42
x.to_s
x.to_i
# to_h with block transforms entries — not redundant
value.to_h.to_h { |k, v| [k, transform(v)] }
