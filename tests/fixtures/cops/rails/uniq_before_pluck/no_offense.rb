Album.distinct.pluck(:band_name)
User.distinct.pluck(:email)
[1, 2, 2, 3].uniq
Album.pluck(:band_name)
Album.select(:band_name).distinct
items.pluck(:name).uniq
