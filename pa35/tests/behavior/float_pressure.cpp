// More live floating values than XMM registers across a loop with a
// floating select (O1+ virtual registers: the select's phi transfers move
// XMM values, and some values spill to frame slots and reload); float and
// double, with integer conversions in the loop.
#include <cstdio>
#include <cstdint>
__attribute__((noinline)) float many(const float* a, int n)
{
	float s0 = 0, s1 = 1, s2 = 2, s3 = 3, s4 = 4, s5 = 5, s6 = 6, s7 = 7, s8 = 8, s9 = 9, s10 = 10, s11 = 11, s12 = 12;
	for (int i = 0; i < n; ++i)
	{
		s0 += a[i];
		s1 *= a[i] * 0.5f + 1;
		s2 -= a[i] / 3.0f;
		s3 += s0 * 0.25f;
		s4 = s4 * 0.9f + s1 * 0.1f;
		s5 += s2 / (s3 + 100.0f);
		s6 -= s4 * 0.01f;
		s7 += s5 > s6 ? s5 : s6;
		s8 = s8 * 0.99f + s7 * 0.001f;
		s9 += s8 - s0;
		s10 *= 1.0001f;
		s11 -= s10 * 0.0001f;
		s12 += s11 + s9 * 0.0001f;
	}
	return s0 + s1 + s2 + s3 + s4 + s5 + s6 + s7 + s8 + s9 + s10 + s11 + s12;
}
__attribute__((noinline)) double dmany(const double* a, int n)
{
	double s0 = 0, s1 = 1, s2 = 2, s3 = 3, s4 = 4, s5 = 5, s6 = 6, s7 = 7, s8 = 8, s9 = 9, s10 = 10, s11 = 11,
		s12 = 12, s13 = 13, s14 = 14;
	for (int i = 0; i < n; ++i)
	{
		s0 += a[i];
		s1 *= a[i] * 0.5 + 1;
		s2 -= a[i] / 3.0;
		s3 += s0 * 0.25;
		s4 = s4 * 0.9 + s1 * 0.1;
		s5 += s2 / (s3 + 100.0);
		s6 -= s4 * 0.01;
		s7 += s5 > s6 ? s5 : s6;
		s8 = s8 * 0.99 + s7 * 0.001;
		s9 += s8 - s0;
		s10 *= 1.0001;
		s11 -= s10 * 0.0001;
		s12 += s11 + s9 * 0.0001;
		s13 = -s13 + static_cast<double>(i);
		s14 += static_cast<double>(static_cast<int64_t>(s13)) + static_cast<float>(s12);
	}
	return s0 + s1 + s2 + s3 + s4 + s5 + s6 + s7 + s8 + s9 + s10 + s11 + s12 + s13 + s14;
}
int main()
{
	float a[100];
	double d[100];
	for (int i = 0; i < 100; ++i)
	{
		a[i] = static_cast<float>(i % 7) * 0.37f - 1.0f;
		d[i] = static_cast<double>(i % 11) * 0.21 - 1.0;
	}
	const float r = many(a, 100);
	const double q = dmany(d, 100);
	uint32_t bits;
	uint64_t qbits;
	__builtin_memcpy(&bits, &r, 4);
	__builtin_memcpy(&qbits, &q, 8);
	std::printf("%08x %016llx\n", bits, static_cast<unsigned long long>(qbits));
	return 0;
}
