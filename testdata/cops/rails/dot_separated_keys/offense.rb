I18n.t(:foo, scope: [:bar, :baz])
             ^^^^^^^^^^^^^^^^^^^^ Rails/DotSeparatedKeys: Use dot-separated keys instead of the `:scope` option.

I18n.translate(:title, scope: [:users, :show])
                       ^^^^^^^^^^^^^^^^^^^^^^^ Rails/DotSeparatedKeys: Use dot-separated keys instead of the `:scope` option.

t(:blank, scope: [:errors, :messages])
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DotSeparatedKeys: Use dot-separated keys instead of the `:scope` option.
