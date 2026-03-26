!x.present?
^^^^^^^^^^^ Rails/Blank: Use `blank?` instead of `!present?`.

!name.present?
^^^^^^^^^^^^^^ Rails/Blank: Use `blank?` instead of `!present?`.

!user.email.present?
^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `blank?` instead of `!present?`.

x.nil? || x.empty?
^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `x.blank?` instead of `x.nil? || x.empty?`.

name.nil? || name.empty?
^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `name.blank?` instead of `name.nil? || name.empty?`.

foo == nil || foo.empty?
^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `foo.blank?` instead of `foo == nil || foo.empty?`.

something unless foo.present?
          ^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `if foo.blank?` instead of `unless foo.present?`.

something unless present?
          ^^^^^^^^^^^^^^^ Rails/Blank: Use `if blank?` instead of `unless present?`.

unless foo.present?
^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `if foo.blank?` instead of `unless foo.present?`.
  something
end

!foo || foo.empty?
^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `foo.blank?` instead of `!foo || foo.empty?`.

!methods || methods.empty?
^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `methods.blank?` instead of `!methods || methods.empty?`.

!url || url.empty?
^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `url.blank?` instead of `!url || url.empty?`.

return self if nil? || empty?
               ^^^^^^^^^^^^^^ Rails/Blank: Use `blank?` instead of `nil? || empty?`.
return [] if nil? || empty?
             ^^^^^^^^^^^^^^ Rails/Blank: Use `blank?` instead of `nil? || empty?`.

if elements.nil? or elements.empty? then
   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `elements.blank?` instead of `elements.nil? or elements.empty?`.

return if name.nil? or name.empty?
          ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `name.blank?` instead of `name.nil? or name.empty?`.

if elements.nil? or elements.empty? then
   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `elements.blank?` instead of `elements.nil? or elements.empty?`.

return if name.nil? or name.empty?
          ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `name.blank?` instead of `name.nil? or name.empty?`.

words.shift if words[0].nil? or words[0].empty?
               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `words[0].blank?` instead of `words[0].nil? or words[0].empty?`.

abbrev = __is_abbrev(line) unless line.nil? || line.empty?
                                  ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `line.blank?` instead of `line.nil? || line.empty?`.

break if line.nil? or line.empty?
         ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `line.blank?` instead of `line.nil? or line.empty?`.

break if rnext.nil? or rnext.empty? or rline.nil? or rline.empty?
         ^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `rnext.blank?` instead of `rnext.nil? or rnext.empty?`.

not @params or @params.empty?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Blank: Use `@params.blank?` instead of `not @params or @params.empty?`.
