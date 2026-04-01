foo.keys.each { |k| p k }
    ^^^^^^^^^ Style/HashEachMethods: Use `each_key` instead of `keys.each`.
foo.values.each { |v| p v }
    ^^^^^^^^^^^ Style/HashEachMethods: Use `each_value` instead of `values.each`.
{}.keys.each { |k| p k }
   ^^^^^^^^^ Style/HashEachMethods: Use `each_key` instead of `keys.each`.
{}.values.each { |k| p k }
   ^^^^^^^^^^^ Style/HashEachMethods: Use `each_value` instead of `values.each`.
opts.each { |key, _directory| p key }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `_directory` block argument.
settings.each { |key, _| p key }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `_` block argument.
data.each { |_k, val| p val }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashEachMethods: Use `each_value` instead of `each` and remove the unused `_k` block argument.

grouped_assessments.each do |(_root_id, _instance_id), assessment_ids|
^ Style/HashEachMethods: Use `each_value` instead of `each` and remove the unused `(_root_id, _instance_id)` block argument.
  p assessment_ids
end

line_num_to_location
^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `(_index_of_newline, _col)` block argument.
  .select { |line_number, (index_of_newline, _col)| index_of_newline.positive? }
  .reject { |line_number, (index_of_newline, _col)| line_number.zero? }
  .each { |line_number, (_index_of_newline, _col)| p line_number }

line_num_to_location.select { |line_number, (index_of_newline, _col)| range.include? index_of_newline }
^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `(_index_of_newline, _col)` block argument.
                    .each { |line_number, (_index_of_newline, _col)| p line_number }

line_num_to_location.select { |line_number, (index_of_newline, _col)| invalid_boundary.include? index_of_newline }
^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `(_index_of_newline, _col)` block argument.
                    .each { |line_number, (_index_of_newline, _col)| p line_number }

wrappings.each do |line_num, (range, _last_col, meta)|
^ Style/HashEachMethods: Use `each_value` instead of `each` and remove the unused `line_num` block argument.
  p range
  p meta
end

active_admin_config.scoped_collection_actions.each do |key, options={}|
^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `options={}` block argument.
  p key
end

summary.urls.keys.each { |k| summary.urls[k.gsub(/&#46;/, ".").sub(%r{^https?://}, "").sub(/^www./, "")] = summary.urls.delete(k) }
             ^^^^^^^^^ Style/HashEachMethods: Use `each_key` instead of `keys.each`.

secrets_with_metadata(prefixed_secrets(secrets, from: from)).each do |secret, (project, secret_name, secret_version)|
^ Style/HashEachMethods: Use `each_value` instead of `each` and remove the unused `secret` block argument.
  p project
  p secret_name
  p secret_version
end

names.chain(renames).each do |name, key = name|
^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `key = name` block argument.
  p name
end

names.chain(renames).each { |key, new_key = key| p key }
^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `new_key = key` block argument.

encrypted_key_data.each do |key_descriptor, key_options = {}|
^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `key_options = {}` block argument.
  p key_descriptor
end

self.class.stubs.each do |path, &block|
^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `&block` block argument.
  p path
end

env.filtered_gems(gemfile.gems).each do |name, *|
^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `*` block argument.
  p name
end

configurations.each do |key, options={}|
^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `options={}` block argument.
  p key
end

keys.each do |key, value|
^ Style/HashEachMethods: Use `each_key` instead of `each` and remove the unused `value` block argument.
  p key
end
