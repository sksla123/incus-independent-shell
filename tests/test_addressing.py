import sys
import unittest

sys.path.insert(0, 'src')
from incus_only_shell.addressing import AddressPlan


class AddressingTests(unittest.TestCase):
    def test_management_123(self):
        p = AddressPlan(123, 3, 10, '10.100')
        self.assertEqual(p.ipv4_for_slot(1), '10.100.11.23')
        self.assertEqual(p.ipv4_for_slot(2), '10.100.21.23')
        self.assertEqual(p.ipv4_for_slot(3), '10.100.31.23')
        self.assertEqual(p.port_for(1, 0), 10123)
        self.assertEqual(p.port_for(1, 9), 19123)
        self.assertEqual(p.port_for(3, 9), 39123)

    def test_boundaries(self):
        p0 = AddressPlan(0, 3, 10, '10.100')
        self.assertEqual(p0.ipv4_for_slot(1), '10.100.10.0')
        self.assertEqual(p0.port_for(1, 0), 10000)
        p9 = AddressPlan(999, 3, 10, '10.100')
        self.assertEqual(p9.ipv4_for_slot(3), '10.100.39.99')
        self.assertEqual(p9.port_for(3, 9), 39999)


if __name__ == '__main__':
    unittest.main()
