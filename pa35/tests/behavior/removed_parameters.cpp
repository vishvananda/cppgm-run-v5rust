// Read-only pointer parameters (host O2+, per parameter index) follow the
// parameters interprocedural specialization keeps: once an unused leading
// parameter leaves the signature, the bit it had must not describe the
// pointer the callee writes through, or the caller passes that callee a
// copy of its local and loses the write.
#include <cstdio>

struct Ctx
{
	int limit;
};

static __attribute__((noinline)) void compute(const Ctx& ctx, int* out, int seed)
{
	(void)ctx;   // used only by an assertion in a checked build
	*out = seed * 3;
}

__attribute__((noinline)) int run(int seed)
{
	Ctx ctx = { 10 };
	int result = -1;
	compute(ctx, &result, seed);
	return result;
}

static __attribute__((noinline)) void bump(int unused, const int* from, int* to, int* also)
{
	*to = *from + 1;
	*also += *from;
}

__attribute__((noinline)) int caller(int a)
{
	int r = a;
	int w = 0;
	int x = 5;
	bump(7, &r, &w, &x);
	return w * 1000 + x;
}

int main(int argc, char**)
{
	std::printf("%d\n", run(argc + 4));
	std::printf("%d\n", caller(argc + 40));
	return 0;
}
