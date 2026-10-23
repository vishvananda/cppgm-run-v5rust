struct I { int n; };
struct F { float n; };
struct D { double n; };
int read(int, ...);

extern "C" int host_read()
{
  I i = { 1 };
  F f = { 2.0f };
  D d = { 3.0 };
  return read(0, i, 7, f, d) ? 0 : 1;
}
