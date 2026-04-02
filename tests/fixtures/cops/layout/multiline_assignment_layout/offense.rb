blarg = if true
^^^^^^^^^^^^^^^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
         'yes'
       else
         'no'
       end

result = if condition
^^^^^^^^^^^^^^^^^^^^^^^^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
           do_thing
         else
           other_thing
         end

value = case x
^^^^^^^^^^^^^^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
        when :a
          1
        else
          2
        end

memoized ||= begin
^^^^^^^^^^^^^^^^^^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
               build_value
             end

result = fetch_records do
^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
           build_record
         end

filtered_fields[k] = v.map do |elem|
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.

logger.formatter = proc do |_severity, _datetime, _progname, learning_arr|
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.

merged_spec['servers'] = merged_spec['servers'].select do |server|
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.

logger.formatter = proc do |severity, _datetime, _progname, msg|
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.

pi.custom_completions = proc do
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.

dec_msg[:type_desc] = case key
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
                      when 'ALN'
                        'Human-readable text'
                      end

dec_msg[:type_desc] = case key
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
                      when 'ALN'
                        'Human-readable text'
                      end

spec.executables = spec.files.grep(%r{^bin/}) do |f|
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
  File.basename(f)
end

work_package.start_date ||= if parent_start_earlier_than_due?
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
                              work_package.parent.start_date
                            elsif Setting.work_package_startdate_is_adddate?
                              Time.zone.today
                            end

@ancestors[work_package] ||= begin
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
  parent = parent_of(work_package)
end

style += if User.current.pref.dark_high_contrast_theme?
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
           <<~CSS.squish
             border: 1px solid black;
           CSS
         else
           <<~CSS.squish
             border: 1px solid white;
           CSS
         end

left, right = if condition
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
                one
              else
                two
              end
