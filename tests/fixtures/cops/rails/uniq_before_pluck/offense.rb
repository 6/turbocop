Album.pluck(:band_name).uniq
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UniqBeforePluck: Use `distinct` before `pluck`.
User.pluck(:email).uniq
^^^^^^^^^^^^^^^^^^^^^^^ Rails/UniqBeforePluck: Use `distinct` before `pluck`.
Post.pluck(:title).uniq
^^^^^^^^^^^^^^^^^^^^^^^ Rails/UniqBeforePluck: Use `distinct` before `pluck`.
