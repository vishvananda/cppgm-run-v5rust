// Switches whose case values lie within 64 of each other (host objects:
// bit tests after one range check): several values per target, a target
// with one value, signed narrow selectors, and 64-bit values at the edge
// of the range.
#include <cstdio>
#include <cstdint>
enum class Op : uint8_t { A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z };
__attribute__((noinline)) bool writes(Op op)
{
	switch (op)
	{
	case Op::C: case Op::E: case Op::F: case Op::K: case Op::M: case Op::P: case Op::T:
		return true;
	case Op::Q:
		return op == Op::Q;
	default:
		return false;
	}
}
__attribute__((noinline)) int width(int8_t k)
{
	switch (k)
	{
	case -3: return 1;
	case -2: case -1: return 2;
	case 0: case 1: return 4;
	case 2: case 3: case 10: return 8;
	case 20: return 16;
	default: return 0;
	}
}
__attribute__((noinline)) int big(int64_t k)
{
	switch (k)
	{
	case INT64_MIN: return 1;
	case INT64_MIN + 1: case INT64_MIN + 5: return 2;
	case INT64_MIN + 2: case INT64_MIN + 9: return 3;
	default: return 0;
	}
}
int main()
{
	unsigned h = 0;
	for (int i = 0; i < 26; ++i)
		h = h * 3 + writes(static_cast<Op>(i));
	for (int k = -10; k < 30; ++k)
		h = h * 5 + static_cast<unsigned>(width(static_cast<int8_t>(k)));
	for (int64_t d = 0; d < 12; ++d)
		h = h * 7 + static_cast<unsigned>(big(INT64_MIN + d)) + static_cast<unsigned>(big(INT64_MAX - d));
	std::printf("%u\n", h);
	return 0;
}
