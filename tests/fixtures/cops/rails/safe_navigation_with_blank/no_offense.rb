do_something if foo.blank?
do_something unless foo.blank?
x if foo&.present?
y unless bar.nil?
z = foo&.blank?
foo.blank?
