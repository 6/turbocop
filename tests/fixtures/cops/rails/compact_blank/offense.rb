arr.reject { |x| x.blank? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/CompactBlank: Use `compact_blank` instead.
arr.select { |x| x.present? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/CompactBlank: Use `compact_blank` instead.
collection.reject { |e| e.blank? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/CompactBlank: Use `compact_blank` instead.
collection.reject(&:blank?)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/CompactBlank: Use `compact_blank` instead.
collection.select(&:present?)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/CompactBlank: Use `compact_blank` instead.
hash.delete_if { |_k, v| v.blank? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/CompactBlank: Use `compact_blank!` instead.
hash.delete_if(&:blank?)
^^^^^^^^^^^^^^^^^^^^^^^^ Rails/CompactBlank: Use `compact_blank!` instead.
