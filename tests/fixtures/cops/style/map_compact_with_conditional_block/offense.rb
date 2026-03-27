# If with no else (implicit nil) — modifier form
ary.map { |x| x if x > 1 }.compact
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
list.map { |item| item if item.valid? }.compact
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
[1, 2, 3].map { |n| n if n.odd? }.compact
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.

# If with no else — block form
ary.map do |x|
^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  if x > 1
    x
  end
end.compact

# Unless modifier form (reject pattern)
ary.map { |item| item unless item.bar? }.compact
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.

# Unless block form (reject pattern)
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  unless item.bar?
    item
  end
end.compact

# If with else=next
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  if item.bar?
    item
  else
    next
  end
end.compact

# If with then=next (reject pattern)
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  if item.bar?
    next
  else
    item
  end
end.compact

# Ternary: select pattern
foo.map { |item| item.bar? ? item : next }.compact
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.

# Ternary: reject pattern
foo.map { |item| item.bar? ? next : item }.compact
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.

# Guard clause: next if (reject)
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  next if item.bar?

  item
end.compact

# Guard clause: next unless (select)
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  next unless item.bar?

  item
end.compact

# Guard clause: next item if (select with value)
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  next item if item.bar?
end.compact

# Guard clause: next item unless (reject with value)
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  next item unless item.bar?
end.compact

# next item if + nil (select with value and nil return)
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  next item if item.bar?

  nil
end.compact

# next item unless + nil (reject with value and nil return)
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  next item unless item.bar?

  nil
end.compact

# If with next item in then branch and nil in else (select)
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  if item.bar?
    next item
  else
    nil
  end
end.compact

# If with nil in then branch and next item in else (reject)
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  if item.bar?
    nil
  else
    next item
  end
end.compact

# filter_map with if/next
ary.filter_map do |item|
    ^^^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Replace `filter_map { ... }` with `select` or `reject`.
  if item.bar?
    item
  else
    next
  end
end

# filter_map with modifier if
ary.filter_map { |item| item if item.bar? }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Replace `filter_map { ... }` with `select` or `reject`.

# Guard clause: next nil if + item (select with nil guard)
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  next nil if item.bar?

  item
end.compact

# Guard clause: next nil unless + item (reject with nil guard)
ary.map do |item|
^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Use `filter_map` instead of `map { ... }.compact`.
  next nil unless item.bar?

  item
end.compact

# filter_map in multi-line method chain (receiver on different line)
foo.bar
  .filter_map { |x| x if x.valid? }
   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapCompactWithConditionalBlock: Replace `filter_map { ... }` with `select` or `reject`.
