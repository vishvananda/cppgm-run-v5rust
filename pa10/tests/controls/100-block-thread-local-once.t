// N3485 6.7p4: a block-scope thread_local with a dynamic initializer is
// initialized once per thread, not on every pass; expect exit 0
static int inits;
static int next() { return ++inits; }
static int f() { thread_local int k = next(); return k; }
int main() { f(); f(); return (f() == 1 && inits == 1) ? 0 : 1; }
