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

dec_msg[:type_desc] = case key
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.

spec.executables = spec.files.grep(%r{^bin/}) do |f|
^ Layout/MultilineAssignmentLayout: Right hand side of multi-line assignment is on the same line as the assignment operator `=`.
