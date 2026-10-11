// Local tables copied from constants and only read (std::to_string's digit
// pairs, a local constexpr array indexed at run time) read the constant
// instead (O2+ constant_slots); a table written after its copy, or whose
// address is passed on, keeps its copy. Signed/unsigned accesses of one
// local (`int r; r += unsigned`) promote it.
#include <cstdio>
#include <string>
__attribute__((noinline)) void touch(char* p) { p[1] ^= 1; }
// A counted loop over a readonly table whose exit reads its sum (O3 table
// search unrolling: the final pass leaves to a copy of that exit).
static constexpr char kTable[11] = "9876543210";
__attribute__((noinline)) int sum_table(unsigned k)
{
	int r = 0;
	for (unsigned i = 0; i < 10; ++i)
		r += kTable[(i + k) % 10];
	return r;
}
__attribute__((noinline)) int tables(unsigned k)
{
	constexpr char digits[11] = "0123456789";
	char written[11] = "abcdefghij";
	written[k % 10] = 'z';
	char passed[11] = "ABCDEFGHIJ";
	touch(passed);
	int r = 0;
	for (unsigned i = 0; i < 10; ++i)
		r += static_cast<unsigned>(digits[(i + k) % 10] + written[i] + passed[(k + i) % 10]);
	return r;
}
int main()
{
	unsigned long t = 0;
	for (unsigned i = 0; i < 20000; i += 7)
	{
		std::string s = std::to_string(i) + std::to_string(-static_cast<int>(i)) + std::to_string(i * 13ull);
		for (char c : s)
			t = t * 31 + static_cast<unsigned char>(c);
		t += static_cast<unsigned long>(tables(i)) + static_cast<unsigned long>(sum_table(i));
	}
	std::printf("%lu\n", t);
	return 0;
}
