'255.255.255.255'
^^^^^^^^^^^^^^^^^ Style/IpAddresses: Do not hardcode IP addresses.
'2001:0db8:85a3:0000:0000:8a2e:0370:7334'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IpAddresses: Do not hardcode IP addresses.
'2001:db8::1'
^^^^^^^^^^^^^ Style/IpAddresses: Do not hardcode IP addresses.
'::1'
^^^^^ Style/IpAddresses: Do not hardcode IP addresses.
'1::'
^^^^^ Style/IpAddresses: Do not hardcode IP addresses.
'2001:db8:85a3::8a2e:370:7334'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IpAddresses: Do not hardcode IP addresses.
'::ffff:192.168.1.1'
^^^^^^^^^^^^^^^^^^^^^ Style/IpAddresses: Do not hardcode IP addresses.
# Link-local IPv6 with zone IDs (fe80 prefix + %zone suffix)
'fe80::1%lo0'
^^^^^^^^^^^^^ Style/IpAddresses: Do not hardcode IP addresses.
'fe80::200:11ff:fe22:1122%5'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IpAddresses: Do not hardcode IP addresses.
'fe80:0:0:0:0:0:0:1%eth0'
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IpAddresses: Do not hardcode IP addresses.

# Compressed IPv6 with more than 7 explicit groups (Ruby's Resolv regex permits this)
data_sample["owner"] = "1::3:4:5:6:7:8:9"
                       ^^^^^^^^^^^^^^^^^^^^ Style/IpAddresses: Do not hardcode IP addresses.
