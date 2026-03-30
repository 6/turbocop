x = 1234
y = 10_000
z = 1_234_567
w = 0xFF
v = 0b1010
u = 0o777
t = 0xDEADBEEF
s = 123
r = 1
# Implicit octal literals (leading 0) should not be flagged
a = 00644
b = 00444
c = 02744
d = 00744
e = 0100644
# Float literals with proper grouping or below min_digits
f = 1_000.0
g = 1234.56
h = 1.0e10
i = 10_000_00
j = 123_456_789_00
k = 819_2
