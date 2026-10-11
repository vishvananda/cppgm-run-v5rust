// N3485 15.3p1, 1.9p1: a handler that rejoins the code after its try-block
// adds to the running total in memory; the read after the loop sees the
// handler's additions at every optimization level, not the value the
// try-block last held in a register
__attribute__((noinline)) int thrower(int x)
{
	if (x > 5)
		throw x;
	return x;
}
int main()
{
	long total = 0;
	for (int a = 0; a < 9; ++a)
	{
		try
		{
			total += thrower(a);
		}
		catch (int e)
		{
			total += 1000 * e;
		}
	}
	return total == 21015 ? 0 : 1;
}
