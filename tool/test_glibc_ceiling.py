import io
import os
import unittest
from contextlib import redirect_stdout

from glibc_ceiling import failed, highest, main, report, version

# Lines as objdump -T prints them, the v0.6.0 x86_64 binary's kind.
SYMBOLS = """
0000000000000000      DF *UND*  0000000000000000 (GLIBC_2.2.5) free
0000000000000000      DF *UND*  0000000000000000 (GLIBC_2.4)  __stack_chk_fail
0000000000000000      DF *UND*  0000000000000000 (GLIBC_2.35) __rseq_offset
0000000000000000      DF *UND*  0000000000000000 (GLIBCXX_3.4.29) _ZNSt7__cxx1112basic_stringIcSt11char_traitsIcESaIcEE10_M_replaceEmmPKcm
0000000000000000      DF *UND*  0000000000000000 (GLIBC_PRIVATE) _dl_find_object
0000000000000000      DF *UND*  0000000000000000 (GCC_3.0)    _Unwind_Resume
"""


class GlibcCeilingTest(unittest.TestCase):
    def test_versions_compare_as_numbers(self):
        self.assertLess(version("2.4"), version("2.35"))
        self.assertLess(version("2.2.5"), version("2.3"))
        self.assertEqual(version("2.35"), (2, 35))
        self.assertIsNone(version("PRIVATE"))
        self.assertIsNone(version("2.35."))

    def test_the_highest_glibc_leaves_out_libstdcxx_and_private(self):
        self.assertEqual(highest(SYMBOLS), "2.35")

    def test_a_newer_glibc_is_the_highest_however_it_sorts_as_text(self):
        self.assertEqual(highest(SYMBOLS + "(GLIBC_2.43) sqrtf\n"), "2.43")
        self.assertEqual(highest("(GLIBC_2.9) a\n(GLIBC_2.17) b\n"), "2.17")

    def test_no_glibc_version_is_none(self):
        self.assertIsNone(highest("(GLIBCXX_3.4) x\n(GCC_3.0) y\n"))

    def test_the_allowed_glibc_or_an_older_passes(self):
        lines = report("oldbrainfix", "2.35", "2.35")
        self.assertFalse(failed(lines))
        self.assertEqual(lines, ["oldbrainfix needs glibc 2.35, at most 2.35"])
        self.assertFalse(failed(report("oldbrainfix", "2.4", "2.35")))

    def test_a_newer_glibc_fails(self):
        # As the v0.6.0 build on ubuntu-latest (Ubuntu 26.04) needed.
        lines = report("oldbrainfix", "2.43", "2.35")
        self.assertTrue(failed(lines))
        self.assertIn("needs glibc 2.43, newer than 2.35", lines[0])

    def test_a_binary_naming_no_glibc_fails(self):
        self.assertTrue(failed(report("oldbrainfix", None, "2.35")))

    def test_a_binary_that_cannot_be_read_fails(self):
        missing = os.path.join(os.path.dirname(__file__), "no-such-binary")
        with redirect_stdout(io.StringIO()) as out:
            self.assertEqual(main(["glibc_ceiling.py", missing, "2.35"]), 1)
        self.assertIn("::error::could not read the symbols", out.getvalue())

    def test_a_wrong_call_is_refused(self):
        with redirect_stdout(io.StringIO()):
            self.assertEqual(main(["glibc_ceiling.py", "oldbrainfix"]), 2)
            self.assertEqual(main(["glibc_ceiling.py", "oldbrainfix", "two"]), 2)


if __name__ == "__main__":
    unittest.main()
