hash[:key]
array[index]
foo[0]
bar[1] = 2
hash["string"]
nested[1][2]
mapping[:users][ record['name'] ] = normalized_name
codes[ response[:output][:result] ]
user['items'][ record['id'] ]
user['items'][ record['id'].to_s ]
records[ key.downcase.to_sym ] = if condition
                                    value1
                                  else
                                    value2
                                  end
memo[ type['name'] ] = {
  'description' => type['text'],
}
current_class_accessor[:table].header_description[ key[1..-1] ] = value
app.extensions[:blog].find { |_key, instance| instance.options[:name] == options[:blog] }[ 1 ]
