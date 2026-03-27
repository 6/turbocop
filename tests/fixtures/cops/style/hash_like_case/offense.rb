case x
^^^^ Style/HashLikeCase: Consider replacing `case-when` with a hash lookup.
when 'a'
  1
when 'b'
  2
when 'c'
  3
end
case y
^^^^ Style/HashLikeCase: Consider replacing `case-when` with a hash lookup.
when :foo
  'bar'
when :baz
  'qux'
when :quux
  'corge'
end

# Array literal bodies of same type (recursive_basic_literal)
case status
^^^^ Style/HashLikeCase: Consider replacing `case-when` with a hash lookup.
when :success
  ["#BackupSuccess"]
when :failure
  ["#BackupFailure"]
when :pending
  ["#BackupPending"]
end

# Hash bodies may contain nested nil values
case @type
^ Style/HashLikeCase: Consider replacing `case-when` with a hash lookup.
when 'agent_person'
  {
    'primary_name' => 'surname',
    'title' => 'title',
    'qualifier' => 'qualifier'
  }
when 'agent_software'
  {
    'software_name' => nil,
    'version' => nil,
    'manufacturer' => nil
  }
when 'agent_corporate_entity'
  {
    'primary_name' => 'primary_name',
    'subordinate_name_1' => 'subordinate_name_1',
    'number' => 'numeration'
  }
end

# Regexp literals count as recursive basic literals
case Current.org.country_code
^ Style/HashLikeCase: Consider replacing `case-when` with a hash lookup.
when "CH"; /\ACH\d{2}3[01]\d{3}[a-z0-9]{12}\z/i
when "FR"; /\AFR\d{12}[a-z0-9]{11}\d{2}\z/i
when "DE"; /\ADE\d{20}\z/i
end

# Array bodies may contain nested nil values
case type
^ Style/HashLikeCase: Consider replacing `case-when` with a hash lookup.
when :"180x180"
  [180, 180]
when :sample
  [850, nil]
when :full
  [nil, nil]
end
