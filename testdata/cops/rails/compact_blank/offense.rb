arr.reject { |x| x.blank? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/CompactBlank: Use `compact_blank` instead of `reject { |e| e.blank? }`.
arr.select { |x| x.present? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/CompactBlank: Use `compact_blank` instead of `select { |e| e.present? }`.
collection.reject { |e| e.blank? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/CompactBlank: Use `compact_blank` instead of `reject { |e| e.blank? }`.
collection.select { |e| e.present? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/CompactBlank: Use `compact_blank` instead of `select { |e| e.present? }`.
