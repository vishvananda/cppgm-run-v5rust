// N3485 4.7p2, 15.3p1: a variable assigned a converted value before a call
// that throws holds the converted value in its handler: the unsigned char
// keeps the low 8 bits, the int the low 32 bits
volatile unsigned long long vx = 0xF000001FFull;
volatile int vt = 1;
__attribute__((noinline)) void thrower()
{
	if (vt)
		throw 1;
}
__attribute__((noinline)) int byte_value(unsigned long long x)
{
	unsigned char b;
	try
	{
		b = static_cast<unsigned char>(x);
		thrower();
	}
	catch (...)
	{
		return b * 3;
	}
	return -1;
}
__attribute__((noinline)) long long word_value(unsigned long long x)
{
	int n;
	try
	{
		n = static_cast<int>(x & 0xFFFFFFFFull);
		thrower();
	}
	catch (...)
	{
		return static_cast<long long>(n) + 1;
	}
	return -1;
}
int main()
{
	return byte_value(vx) == 765 && word_value(vx) == 512 ? 0 : 1;
}
