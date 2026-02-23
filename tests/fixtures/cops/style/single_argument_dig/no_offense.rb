{ key: 'value' }[:key]
[1, 2, 3][0]
{ key1: { key2: 'value' } }.dig(:key1, :key2)
[1, [2, [3]]].dig(1, 1)
keys = %i[key1 key2]
{ key1: { key2: 'value' } }.dig(*keys)
