MyTable.arel_table["*"]
                   ^^^ Rails/ArelStar: Use `Arel.star` instead of `"*"` for expanded column lists.
User.arel_table["*"]
                ^^^ Rails/ArelStar: Use `Arel.star` instead of `"*"` for expanded column lists.
Post.arel_table["*"].count
                ^^^ Rails/ArelStar: Use `Arel.star` instead of `"*"` for expanded column lists.
