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
if !(/[A-Z]/ =~ kind)
end
meta = j.metacol_id && !(/metacol/ =~ params[:type]) ? (' (' + link_to(j.metacol_id, j.metacol) + ')') : ''
(xml.include?("<feed") && xml.include?("Atom") && xml.include?("feedburner") && !(/<rss|<rdf/ =~ xml)) || false

rows.map do |j|
  sub = User.find(j.submitted_by)
  doer = User.find_by(id: j.user_id)
  group = Group.find(j.group_id)
  name = j.path.split('/').last.split('.').first
  meta = j.metacol_id && !(/metacol/ =~ params[:type]) ? (' (' + link_to(j.metacol_id, j.metacol) + ')') : ''
end

if ventilate
  result.map do |l|
    (l.start_with? '//') || !(STOP_PUNCTUATION.any? { |punc| l.include? punc }) ? l : (l.gsub StopPunctRx, LF)
  end.join LF
else
  result.join LF
end

def empty?
  !any?
end

def without_platforms
  select { |k, v| !v.has_platforms? }
end
