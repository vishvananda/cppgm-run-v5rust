// expect exit 0
// N3485 13.3.1.1.2: member selection precedes surrogate selection; a
// nonviable or ambiguous member set still permits the inherited surrogate.
typedef int (*unary)(int);
typedef int (*binary)(int, int);
int add(int a, int b) { return a + b; }
int identity(int a) { return a; }
struct Base { operator binary() { return add; } };
struct Nonviable : Base { int operator()(int) { return 90; } };
struct Preferred {
  operator unary() { return identity; }
  int operator()(int) { return 91; }
};
struct Ambiguous {
  operator unary() { return identity; }
  int operator()(long) { return 92; }
  int operator()(unsigned long) { return 93; }
};
int main() {
  Nonviable n;
  Preferred p;
  Ambiguous a;
  return (((n)))(2, 3) != 5 || p(7) != 91 || a(8) != 8;
}
