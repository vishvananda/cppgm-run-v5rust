#include <cstdarg>
#include <cstdlib>
struct F { float n; };
struct D { double n; };
struct M { double n; int m; };
struct L { long n[3]; };
struct E {};
struct G { int n; G(int, ...); };
struct C { int n; int read(int, ...); void method(); };
static int failed;
static int check(va_list a)
{
  F f = va_arg(a, F);
  D d = va_arg(a, D);
  M m = va_arg(a, M);
  L l = va_arg(a, L);
  return f.n == 2.0f && d.n == 3.0 && m.n == 4.0 && m.m == 5
    && l.n[0] == 6 && l.n[1] == 7 && l.n[2] == 8;
}
int read(int marker, ...)
{
  va_list a;
  va_start(a, marker);
  int result = check(a);
  va_end(a);
  return result;
}
G::G(int marker, ...)
{
  va_list a;
  va_start(a, marker);
  n = check(a);
  va_end(a);
}
int C::read(int marker, ...)
{
  va_list a;
  va_start(a, marker);
  int result = check(a) && n == 9;
  va_end(a);
  return result;
}
int read_empty(int marker, ...)
{
  va_list a;
  va_start(a, marker);
  va_arg(a, E);
  int result = va_arg(a, int) == 10;
  va_end(a);
  return result;
}
void C::method() {}
int read_pointer(int marker, ...)
{
  va_list a;
  va_start(a, marker);
  void (C::*p)() = va_arg(a, void (C::*)());
  va_end(a);
  return p == &C::method;
}
void* operator new(std::size_t size, ...)
{
  va_list a;
  va_start(a, size);
  F f = va_arg(a, F);
  va_end(a);
  if (f.n != 2.0f) failed = 1;
  return std::malloc(size);
}
extern "C" int host_status() { return failed; }
