foo ||= bar

foo ||=
  bar

foo ||= bar.each do |b|
  b.baz
  bb.ax
end

foo ||=
  bar.each do |b|
    b.baz
    b.bax
  end

foo ||= if bar
          baz
        else
          bax
        end

foo ||= begin
  bar
  baz
end

@secure_group_ids ||=
  (self.category.secure_group_ids if self.category && self.category.read_restricted?)
