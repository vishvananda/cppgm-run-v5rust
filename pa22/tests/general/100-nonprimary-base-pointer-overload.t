struct A { int a; };
struct B { int b; };
struct D : A, B { int d; };

int pick(B *) { return 7; }
int pick(void *) { return 2; }

int main() {
  D d;
  return pick(&d) == 7 ? 0 : 1;
}
