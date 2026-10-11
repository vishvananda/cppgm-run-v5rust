struct P { long sig; char opaque[2]; };
struct X { P m = {1, {0}}; };
int main() {
  X x;
  return (x.m.sig == 1 && x.m.opaque[0] == 0 && x.m.opaque[1] == 0) ? 0 : 1;
}
