extern "C" int printf(const char*, ...);
struct D { int v; ~D() { printf("~D %d\n", v); } };
extern thread_local D w;
int read_u2();
