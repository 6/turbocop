case foo
when bar
end

case foo
when bar
  do_something
end

case foo
when bar
  do_something1
  do_something2
end

case foo
when bar, baz
end

# when `then` is on a separate line from `when`
case foo
when bar
  do_something
end

case bookmarkable
when "Work"
  work_bookmarks_path(bookmarkable)
when "ExternalWork"
  external_work_bookmarks_path(bookmarkable)
when "Series"
  series_bookmarks_path(bookmarkable)
end
