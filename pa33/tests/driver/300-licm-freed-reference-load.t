// N3485 3.7.4.2p4, 1.9p1: a loop that runs zero times does not read through
// a reference whose object was deleted before the loop (on every path or on
// one), so moving the load to the preheader may not read the freed storage
// either (these objects are large enough that deleting them unmaps their
// pages)
struct Big
{
	long value;
	const char* name;
	char pad[1 << 20];
};
struct Bigger
{
	long value;
	char pad[1 << 23];
};
struct Huge
{
	long value;
	const char* name;
	char pad[1 << 24];
};
__attribute__((noinline)) void release(Big* big)
{
	delete big;
}
__attribute__((noinline)) void release(Bigger* big)
{
	delete big;
}
__attribute__((noinline)) void release(Huge* big)
{
	delete big;
}
__attribute__((noinline)) long sum_after_delete(Big& big, int n)
{
	long s = 1;
	release(&big);
	for (int i = 0; i < n; ++i)
		s += big.name[i];
	return s;
}
__attribute__((noinline)) void fill_after_delete(Bigger& big, long* out, int n)
{
	release(&big);
	for (int i = 0; i < n; ++i)
		out[i] = big.value;
}
__attribute__((noinline)) long sum_after_branch_delete(Huge& big, int n, int f)
{
	long s = 0;
	if (f)
		release(&big);
	else
		s = big.value;
	for (int i = 0; i < n; ++i)
		s += big.name[i];
	return s;
}
__attribute__((noinline)) long sum_kept(Big& big, int n)
{
	long s = 0;
	for (int i = 0; i < n; ++i)
		s += big.name[i];
	return s;
}
volatile int zero = 0;
int main()
{
	Big* kept = new Big;
	kept->name = "hello";
	if (sum_kept(*kept, zero + 5) != 532)
		return 1;
	Big* a = new Big;
	a->name = "hello";
	if (sum_after_delete(*a, zero) != 1)
		return 2;
	Bigger* b = new Bigger;
	b->value = 42;
	long out[2] = { 7, 7 };
	fill_after_delete(*b, out, zero);
	if (out[0] != 7)
		return 3;
	Huge* c = new Huge;
	c->name = "hello";
	if (sum_after_branch_delete(*c, zero, 1) != 0)
		return 4;
	delete kept;
	return 0;
}
