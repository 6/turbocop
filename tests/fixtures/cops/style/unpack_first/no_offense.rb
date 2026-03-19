'foo'.unpack1('h*')
'foo'.unpack('h*')
'foo'.unpack('h*').last
'foo'.unpack('h*')[1]
'foo'.unpack('h*', 'i*').first
x.first

# Bare unpack without explicit receiver (implicit self) — not flagged by RuboCop
unpack("H*")[0]
unpack('h*').first
