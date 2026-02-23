array.reject { |e| e.nil? }
      ^^^^^^^^^^^^^^^^^^^^^ Style/CollectionCompact: Use `compact` instead of `reject { |e| e.nil? }`.

array.reject!(&:nil?)
      ^^^^^^^^^^^^^^^ Style/CollectionCompact: Use `compact!` instead of `reject!(&:nil?)`.

hash.reject { |k, v| v.nil? }
     ^^^^^^^^^^^^^^^^^^^^^^^^ Style/CollectionCompact: Use `compact` instead of `reject { |e| e.nil? }`.
