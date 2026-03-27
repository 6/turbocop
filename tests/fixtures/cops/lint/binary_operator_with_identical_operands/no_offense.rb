x == y
a && b
x + x
1 << 1
a != b
c >= d
mask & mask

assert { "123\n456\n" == <<-TEXT
123
456
TEXT
}

assert { "123\n456\n" == <<-TEXT }
123
456
TEXT
