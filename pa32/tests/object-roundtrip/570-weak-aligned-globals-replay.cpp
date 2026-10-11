// GNU weak definitions (binding=weak, replaceable=yes) and over-aligned
// globals (an obj<NxA> storage type) keep their object form through the
// -O0 LowIR text: weak symbols that are never inlined or dropped, and data
// at its required alignment.
struct alignas(16) Block { int value; };

__attribute__((weak)) int hook() { return 1; }
__attribute__((weak)) int unused_hook() { return 2; }
__attribute__((weak)) int weak_value = 3;

alignas(32) int table[4] = {1, 2, 3, 4};
int __attribute__((aligned(64))) counter = 5;
Block block = {7};
int zeros[3];

int main()
{
	const unsigned long t = reinterpret_cast<unsigned long>(table);
	const unsigned long c = reinterpret_cast<unsigned long>(&counter);
	const unsigned long b = reinterpret_cast<unsigned long>(&block);
	return hook() + weak_value == 4 && t % 32 == 0 && c % 64 == 0 &&
		b % 16 == 0 && table[2] + counter + block.value + zeros[1] == 15 ? 0 : 1;
}
