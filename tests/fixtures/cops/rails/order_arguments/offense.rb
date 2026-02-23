User.order('name')
           ^^^^^^ Rails/OrderArguments: Prefer `:name` instead.
User.order('name DESC')
           ^^^^^^^^^^^ Rails/OrderArguments: Prefer `name: :desc` instead.
User.order('email ASC')
           ^^^^^^^^^^^ Rails/OrderArguments: Prefer `:email` instead.
order('created_at desc')
      ^^^^^^^^^^^^^^^^^^^ Rails/OrderArguments: Prefer `created_at: :desc` instead.
