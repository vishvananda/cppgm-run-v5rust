// Preserve partial INTEGER return loads alongside full and SSE eightbytes.
struct Twelve { int a, b, c; };
struct __attribute__((packed)) Nine { long a; char b; };
struct __attribute__((packed)) Thirteen { long a; int b; char c; };
struct Mixed { double a; int b; };
__attribute__((noinline)) Twelve twelve(Twelve value) { return value; }
__attribute__((noinline)) Nine nine(Nine value) { return value; }
__attribute__((noinline)) Thirteen thirteen(Thirteen value) { return value; }
__attribute__((noinline)) Mixed mixed(Mixed value) { return value; }
int main() {
  volatile int source = 17;
  Twelve a = {source, -source, source + 1};
  Twelve b = twelve(a);
  if (a.a != b.a || a.b != b.b || a.c != b.c) return 1;
  Nine c = {source, -3};
  Nine d = nine(c);
  if (c.a != d.a || c.b != d.b) return 2;
  Thirteen e = {source, -source, 5};
  Thirteen f = thirteen(e);
  if (e.a != f.a || e.b != f.b || e.c != f.c) return 3;
  Mixed g = {1.25, source};
  Mixed h = mixed(g);
  if (g.a != h.a || g.b != h.b) return 4;
  return 0;
}
