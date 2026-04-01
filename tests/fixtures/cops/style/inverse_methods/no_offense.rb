foo.none?
foo.any?
foo.exclude?(bar)
foo.odd?
foo.select { |x| x > 0 }
foo.reject { |x| x < 0 }
!foo.include?(bar)
!foo.present?
!foo.blank?
!foo.empty?
# Class hierarchy checks — Module#< can return nil, so !(A < B) != (A >= B)
!(routes < AbstractRouter)
!(Foo > Bar)
!(Foo::Bar <= Baz::Qux)
!(klass >= SomeModule)
# Block with guard clause (next) — not flagged
items.select do |x|
  next if x.zero?
  x != 1
end
# Double negation !! — not an inversion, converts to boolean
!!(line =~ /pattern/)
!!(x == true)
!!(foo.any?)
# Safe navigation &. with incompatible methods — can't invert
!foo&.any?
!foo&.none?

def empty?
  !any?
end

def without_platforms
  select { |k, v| !v.has_platforms? }
end
