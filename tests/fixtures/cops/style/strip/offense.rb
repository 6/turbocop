'abc'.lstrip.rstrip
      ^^^^^^^^^^^^^ Style/Strip: Use `strip` instead of `lstrip.rstrip`.

'abc'.rstrip.lstrip
      ^^^^^^^^^^^^^ Style/Strip: Use `strip` instead of `rstrip.lstrip`.

str.lstrip.rstrip
    ^^^^^^^^^^^^^ Style/Strip: Use `strip` instead of `lstrip.rstrip`.

lstrip.rstrip.gsub(/\{\{([^\}]+)\}\}/) { |special|
^ Style/Strip: Use `strip` instead of `lstrip.rstrip`.

lstrip.rstrip
^ Style/Strip: Use `strip` instead of `lstrip.rstrip`.

lstrip.rstrip.gsub(/\{\{([^\}]+)\}\}/) { |special|
^ Style/Strip: Use `strip` instead of `lstrip.rstrip`.

def cleanup_entry(line)
  line[1..-1].gsub(/\[\[([^\|\]]+)\|([^\]]+)\]\]/) { |link|
    $2
  }.
    gsub(/''/, "'").
    gsub(/\]\]/, "").
    gsub(/\[\[/, "").
    gsub(/\&ndash;/, "-").
    gsub(/ +/, ' ').
    lstrip.rstrip.gsub(/\{\{([^\}]+)\}\}/) { |special|
    ^^^^^^^^^^^^^ Style/Strip: Use `strip` instead of `lstrip.rstrip`.
    stuff = $1.split("|")
  }
end
