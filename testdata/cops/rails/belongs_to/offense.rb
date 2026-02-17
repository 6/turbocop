belongs_to :blog, required: false
^^^^^^^^^^ Rails/BelongsTo: You specified `required: false`, in Rails > 5.0 the required option is deprecated and you want to use `optional: true`.
belongs_to :author, required: true
^^^^^^^^^^ Rails/BelongsTo: You specified `required: true`, in Rails > 5.0 the required option is deprecated and you want to use `optional: false`.
belongs_to :category, class_name: 'Tag', required: false
^^^^^^^^^^ Rails/BelongsTo: You specified `required: false`, in Rails > 5.0 the required option is deprecated and you want to use `optional: true`.
