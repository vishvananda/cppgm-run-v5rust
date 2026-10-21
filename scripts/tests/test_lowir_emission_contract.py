#!/usr/bin/env python3
"""Optional data, required roots, and ordered construction-table pairing."""
import unittest
import test_compare_lowir_results as comparison


class EmissionContractTest(unittest.TestCase):
    compare = comparison.LowIrComparisonTest.compare
    MAIN = """function @main() -> i32 [role=entry] {
  block ^entry:
    return i32 0
}
"""

    def check(self, reference, generated, expected):
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode == 0, expected, result.stdout + result.stderr)

    def test_optional_globals_and_required_roots(self):
        for metadata, removable in (
            ('binding=internal', True), ('binding=weak', True),
            ('binding=strong', False),
            ('binding=weak, replaceable=yes', False),
            ('binding=internal, object_root=yes', False),
        ):
            with self.subTest(metadata=metadata):
                data = f'global @item : i32 [{metadata}] = 7\n'
                self.check(data + self.MAIN, self.MAIN, removable)
                self.check(data + self.MAIN, data.replace('= 7', '= 8') + self.MAIN, False)

    def test_unused_declarations_and_refinement(self):
        self.check('declare global @left : i32\n' + self.MAIN,
                   'declare global @right : i32\n' + self.MAIN, True)
        # A declaration may be refined to a definition even if nothing uses it.
        self.check('declare function @extra() -> i32\n' + self.MAIN,
                   'function @extra() -> i32 {\n  block ^entry:\n    return i32 7\n}\n' + self.MAIN, True)

    def test_transitive_optional_data_and_cycles(self):
        data = 'global @item : i32 [binding=internal] = 7\n'
        leaf = """function @leaf() -> i32 [binding=weak] {
  block ^entry:
    %v = load typed i32 @item
    return i32 %v
}
"""
        self.check(data + leaf + self.MAIN, self.MAIN, True)
        self.check(data + leaf.replace('binding=weak', 'binding=strong') + self.MAIN,
                   self.MAIN, False)
        cycle = ('global @a : ptr [binding=internal] = addr @b\n'
                 'global @b : ptr [binding=internal] = addr @a\n')
        self.check(cycle + self.MAIN, self.MAIN, True)
        self.check(cycle.replace('binding=internal', 'binding=strong', 1) + self.MAIN,
                   self.MAIN, False)

    def test_live_data_and_renamed_optional_data_are_compared(self):
        data = 'global @item : i32 [binding=internal] = 7\n'
        main = self.MAIN.replace('    return i32 0',
                                 '    %v = load typed i32 @item\n    return i32 %v')
        self.check(data + main, main, False)
        renamed = (data + main).replace('@item', '@renamed')
        self.check(data + main, renamed, True)
        self.check(data + main, renamed.replace('= 7', '= 8'), False)

    def test_ordered_global_anchors_pair_identical_table_shapes(self):
        leaves = ('global @a : i32 [binding=weak, object=a] = 1\n'
                  'global @b : i32 [binding=weak, object=b] = 2\n')
        tables = ('global @first : ptr [binding=internal] = addr @a\n'
                  'global @second : ptr [binding=internal] = addr @b\n')
        anchor = ('global @anchor [binding=strong, object=anchor] = {\n'
                  '  ptr addr @first\n  ptr addr @second\n}\n')
        reference = leaves + tables + anchor + self.MAIN
        generated = (leaves + '\n'.join(reversed(tables.splitlines())) + '\n' + anchor + self.MAIN)
        for old, new in (('@first', '@other_x'), ('@second', '@other_y'),
                         ('@a', '@renamed_a'), ('@b', '@renamed_b')):
            generated = generated.replace(old, new)
        self.check(reference, generated, True)
        self.check(reference, generated.replace('addr @renamed_a', 'addr @renamed_b'), False)


if __name__ == '__main__':
    unittest.main()
