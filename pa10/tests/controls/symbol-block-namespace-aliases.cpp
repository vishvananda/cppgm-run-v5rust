// expect exit 0
namespace N {
inline int choose(int x) { return x; }
int choose(double) { return 99; }
int before() {
  int choose(int);
  extern int after;
  int (*pointer)(int) = choose;
  return pointer(after);
}
int after = 3;
int forward() { extern int values[]; return values[1]; }
int values[] = {4, 7};
}
int first() { int later(int); return later(1); }
int second() { int later(double); return later(1.0); }
int later(int x) { return x + 1; }
int later(double) { return 3; }
int main() {
  return N::before() == 3 && N::forward() == 7 && first() == 2 &&
    second() == 3 ? 0 : 1;
}
