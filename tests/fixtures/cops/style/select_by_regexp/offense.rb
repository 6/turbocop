array.select { |x| x.match?(/regexp/) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SelectByRegexp: Prefer `grep` to `select` with a regexp match.

array.select { |x| /regexp/.match?(x) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SelectByRegexp: Prefer `grep` to `select` with a regexp match.

array.reject { |x| x =~ /regexp/ }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SelectByRegexp: Prefer `grep_v` to `reject` with a regexp match.

files += self.local_files.select do |file|
         ^ Style/SelectByRegexp: Prefer `grep` to `select` with a regexp match.
  file =~ name
end

ents.select {|ent| re =~ ent }
^ Style/SelectByRegexp: Prefer `grep` to `select` with a regexp match.

ents.reject {|ent| re =~ ent }
^ Style/SelectByRegexp: Prefer `grep_v` to `reject` with a regexp match.

columns_to_modify = ActiveRecord::Base.connection.select_all("SHOW CREATE TABLE #{table}").rows[0][1].split(/\n/).select { |col| col =~ CONVERT_DATA_TYPES }
                    ^ Style/SelectByRegexp: Prefer `grep` to `select` with a regexp match.

@out_from[source].values.flatten.find_all { |edge| edge.match?(event.name) }
^ Style/SelectByRegexp: Prefer `grep` to `find_all` with a regexp match.

options.keys
^ Style/SelectByRegexp: Prefer `grep` to `select` with a regexp match.
       .select { |a| a =~ Regexp.new("\\A#{opt}") }
       .sort
       .map { |a| options[a] }

renderer = Registry.renderer.select { |r| r.match?(obj) }
           ^ Style/SelectByRegexp: Prefer `grep` to `select` with a regexp match.

assert_equal(stdout_lines.reject { / # [a-z]/.match?(_1) }.size, stdout_lines.size)
             ^ Style/SelectByRegexp: Prefer `grep_v` to `reject` with a regexp match.
