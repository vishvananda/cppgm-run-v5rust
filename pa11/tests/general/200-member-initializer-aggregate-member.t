struct P { long sig; char opaque[2]; };
struct X { P m; X() : m{2, {3, 4}} {} };
int main() {
  X x;
  return (x.m.sig == 2 && x.m.opaque[0] == 3 && x.m.opaque[1] == 4) ? 0 : 1;
}
