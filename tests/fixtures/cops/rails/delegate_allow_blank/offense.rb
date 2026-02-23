delegate :name, to: :client, allow_blank: true
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DelegateAllowBlank: `allow_blank` is not a valid option for `delegate`. Did you mean `allow_nil`?

delegate :email, to: :account, allow_blank: false
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DelegateAllowBlank: `allow_blank` is not a valid option for `delegate`. Did you mean `allow_nil`?

delegate :title, :body, to: :post, allow_blank: true
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DelegateAllowBlank: `allow_blank` is not a valid option for `delegate`. Did you mean `allow_nil`?
