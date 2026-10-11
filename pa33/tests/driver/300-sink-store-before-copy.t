// N3485 3.9p2, 1.9p1: copying an object's bytes into another object of its
// type with std::memcpy gives it the same value, also into variables first
// initialized to zero, at every optimization level
#include <cstring>
static const unsigned long long a = 0x1122334455667788ull;
static const unsigned long long b = 0x0102030405060708ull;
static const double c = 2.5;
int main()
{
	unsigned long long first = 0, second = 0, third = 0;
	std::memcpy(&first, &a, 8);
	std::memcpy(&second, &b, 8);
	std::memcpy(&third, &c, 8);
	return first == 0x1122334455667788ull && second == 0x0102030405060708ull && third == 0x4004000000000000ull ? 0 : 1;
}
