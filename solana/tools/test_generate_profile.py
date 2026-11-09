import unittest
from pathlib import Path
from generate_profile import generate, field_expression, literal


class GeneratorTests(unittest.TestCase):
    def test_rejects_changed_reference(self):
        with self.assertRaisesRegex(ValueError, 'source hash'):
            generate('contract Changed {}')

    def test_rejects_unknown_arithmetic_and_modulus(self):
        for expression in ['div(2, 3)', 'mulmod(2, 3, 7)', 'sub(4, 5)', 'mload(123)']:
            with self.subTest(expression=expression), self.assertRaises(ValueError):
                field_expression(expression, {})

    def test_preserves_subtraction_in_the_scalar_field(self):
        self.assertEqual(field_expression('sub(mload(0x60), 2)', {}), f'(-{literal(2)})')

    def test_subtraction_and_squaring_lowering(self):
        self.assertEqual(field_expression('addmod(7, sub(mload(0x60), 4), mload(0x60))', {}), f'sub({literal(7)}, {literal(4)})')
        self.assertEqual(field_expression('mulmod(7, 7, mload(0x60))', {}), f'square({literal(7)})')

    def test_generates_all_six_stages_from_pinned_source(self):
        source = (Path(__file__).resolve().parents[2] / 'zisk-contracts/blake3/PilfflonkVerifier.sol').read_text()
        output = generate(source)
        for name in ['compute_zh', 'compute_zi', 'compute_q', 'compute_z', 'compute_inversions', 'compute_r']:
            self.assertIn(f'fn {name}(', output)


if __name__ == '__main__':
    unittest.main()
