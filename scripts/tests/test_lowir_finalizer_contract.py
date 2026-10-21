#!/usr/bin/env python3
"""Finalizer execution and optional-definition reachability boundaries."""
import unittest
import test_compare_lowir_results as comparison


class FinalizerContractTest(unittest.TestCase):
    compare = comparison.LowIrComparisonTest.compare

    PREFIX = """declare function @construct(%p : ptr) -> void [object=construct]
declare function @destroy(%p : ptr) -> void [object=destroy]
global @first : i32 [binding=strong, object=first] = zero
global @second : i32 [binding=strong, object=second] = zero
function @main() -> i32 [role=entry] {
  block ^entry:
    return i32 0
}
"""
    INIT = """function @init() -> void [role=init] {
  block ^entry:
    %a = addr @first
    call void @construct(%a)
    %b = addr @second
    call void @construct(%b)
    return void
}
"""
    FINI = """function @fini() -> void [role=fini] {
  block ^entry:
    %b = addr @second
    call void @destroy(%b)
    %a = addr @first
    call void @destroy(%a)
    return void
}
"""
    REGISTER = 'declare function @register(%callback : ptr) -> i32 [object=atexit]\n'

    def callbacks(self):
        return ''.join(f"""function @{name}() -> void [unwind=no] {{
  block ^entry:
    %p = addr @{obj}
    call void @destroy(%p)
    return void
}}
""" for name, obj in [('retire_a', 'first'), ('retire_b', 'second')])

    def split(self):
        init = self.INIT.replace('    %b = addr', '    %ra = call i32 @register(@retire_a)\n    %b = addr')
        init = init.replace('    return void', '    %rb = call i32 @register(@retire_b)\n    return void')
        return self.PREFIX + self.REGISTER + init + self.callbacks()

    def check(self, ref, actual, expected):
        result = self.compare(ref, actual)
        self.assertEqual(result.returncode == 0, expected, result.stdout + result.stderr)

    def test_split_callbacks_and_rejection_boundaries(self):
        ref = self.PREFIX + self.INIT + self.FINI
        actual = self.split()
        variants = {
            'valid': (actual, True),
            'missing-registration': (actual.replace('    %ra = call i32 @register(@retire_a)\n', ''), False),
            'reordered': (actual.replace('@register(@retire_a)', '@register(@swap)').replace('@register(@retire_b)', '@register(@retire_a)').replace('@register(@swap)', '@register(@retire_b)'), False),
            'wrong-object': (actual.replace('%p = addr @second', '%p = addr @first'), False),
            'extra-action': (actual.replace('    call void @destroy(%p)', '    call void @destroy(%p)\n    call void @destroy(%p)', 1), False),
            'startup-store': (actual.replace('    %ra = call', '    store i32 3, @first\n    %ra = call', 1), False),
            'callback-store': (actual.replace('    call void @destroy(%p)', '    store i32 3, @first\n    call void @destroy(%p)', 1), False),
            'duplicate': (actual.replace('    %ra = call', '    %r0 = call i32 @register(@retire_a)\n    %ra = call', 1), False),
        }
        for name, (candidate, expected) in variants.items():
            with self.subTest(name=name):
                self.check(ref, candidate, expected)

    def test_registration_only_initializer(self):
        init = """function @init() -> void [role=init] {
  block ^entry:
    %ra = call i32 @register(@retire_a)
    %rb = call i32 @register(@retire_b)
    return void
}
"""
        ref = self.PREFIX + self.FINI
        actual = self.PREFIX + self.REGISTER + init + self.callbacks()
        self.check(ref, actual, True)
        self.check(ref, actual.replace('    %ra = call', '    store i32 3, @first\n    %ra = call', 1), False)

    def test_single_callback_registration_boundaries(self):
        fini = self.FINI.replace('    %b = addr @second\n    call void @destroy(%b)\n', '')
        init = self.INIT.replace('    return void', '    %ra = call i32 @register(@retire_a)\n    return void')
        ref = self.PREFIX + self.INIT + fini
        actual = self.PREFIX + self.REGISTER + init + self.callbacks().split('function @retire_b')[0]
        self.check(ref, actual, True)
        self.check(ref, actual.replace('    %ra = call i32 @register(@retire_a)\n', ''), False)
        self.check(ref, actual.replace('    %ra = call', '    %r0 = call i32 @register(@retire_a)\n    %ra = call'), False)
        self.check(ref, actual.replace('    call void @destroy(%p)', '    store i32 1, @first\n    call void @destroy(%p)'), False)

    def test_skipped_single_and_split_destructors(self):
        for split in (False, True):
            actual = self.split()
            fini = self.FINI
            if not split:
                fini = fini.replace('    %b = addr @second\n    call void @destroy(%b)\n', '')
                actual = actual.replace('    %rb = call i32 @register(@retire_b)\n', '').split('function @retire_b')[0]
            for condition in ('0', '%unknown'):
                with self.subTest(split=split, condition=condition):
                    candidate = actual.replace('  block ^entry:\n    %p = addr @first',
                        f'  block ^entry:\n    branch {condition}, ^destroy, ^done\n  block ^destroy:\n    %p = addr @first')
                    candidate = candidate.replace('    call void @destroy(%p)\n    return void',
                        '    call void @destroy(%p)\n    jump ^done\n  block ^done:\n    return void', 1)
                    if condition == '%unknown':
                        candidate = candidate.replace('    branch %unknown', '    %unknown = load i32 @second\n    branch %unknown')
                    self.check(self.PREFIX + self.INIT + fini, candidate, False)

    def test_integer_bookkeeping_does_not_hide_skipped_calls(self):
        fini = self.FINI.replace('    %b = addr @second\n    call void @destroy(%b)\n', '')
        actual = self.split().replace('    %rb = call i32 @register(@retire_b)\n', '').split('function @retire_b')[0]
        for expression in ('binary add u8 255, 1', 'cmp ult i64 -1, 0', 'cmp ule i64 -1, 0'):
            with self.subTest(expression=expression):
                candidate = actual.replace('  block ^entry:\n    %p = addr @first',
                    f'  block ^entry:\n    %n = {expression}\n    branch %n, ^destroy, ^done\n  block ^destroy:\n    %p = addr @first')
                candidate = candidate.replace('    call void @destroy(%p)\n    return void',
                    '    call void @destroy(%p)\n    jump ^done\n  block ^done:\n    return void', 1)
                self.check(self.PREFIX + self.INIT + fini, candidate, False)

    def test_finalizer_loop_count_and_order_are_checked(self):
        reference = comparison.LowIrComparisonTest.FINALIZER_ARRAY_REFERENCE
        loop = """function @fini() -> void [role=fini] {
  slot $index : i64
  block ^entry:
    store i64 COUNT, $index
    jump ^test
  block ^test:
    %remaining = load i64 $index
    %more = cmp ne i64 %remaining, 0
    branch %more, ^body, ^done
  block ^body:
    %next = binary sub i64 %remaining, 1
    store i64 %next, $index
    %offset = binary mul i64 %next, 4
    %base = addr @items
    %element = index i8 [projection=array_element] %base, %offset
    call void @destroy(%element)
    jump ^test
  block ^done:
    %other = addr @other
    call void @finish(%other)
    return void
}
"""
        prefix = reference.split('\nfunction @fini')[0] + '\n'
        for count in (0, 1, 2, 3, 4, -1):
            with self.subTest(count=count):
                self.check(reference, prefix + loop.replace('COUNT', str(count)), count == 3)

    def test_transitive_optional_definition_graph(self):
        leaf = """function @leaf() -> void [binding=weak] {
  block ^entry:
    return void
}
"""
        parent = """function @parent() -> void [binding=weak] {
  block ^entry:
    call void @leaf()
    return void
}
"""
        root = comparison.REFERENCE
        self.check(root + leaf + parent, root, True)
        self.check(root + parent + leaf, root, True)
        cycle = leaf.replace('    return void', '    call void @parent()\n    return void')
        self.check(root + parent + cycle, root, True)
        self.check(root + parent.replace('binding=weak', 'binding=weak, object_root=yes') + leaf, root, False)
        self.check(root + parent.replace('binding=weak', 'binding=strong') + leaf, root, False)
        live = root.replace('    %sum =', '    call void @parent()\n    %sum =')
        self.check(live + parent + leaf, live, False)


if __name__ == '__main__':
    unittest.main()
