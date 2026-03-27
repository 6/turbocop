# Inline disable on the overflowing parent line should not suppress
# descendant offenses on later lines.
# nitrocop-expect: 7:8 Metrics/BlockNesting: Avoid more than 3 levels of block nesting.
# nitrocop-expect: 8:10 Metrics/BlockNesting: Avoid more than 3 levels of block nesting.
def foo
  if a
    if b
      while current_folder_id
        if folder_resource.exists? # rubocop:disable Metrics/BlockNesting
          if parent_name.include?('organizations/')
            org_domain = true
          end
        end
      end
    end
  end
end
