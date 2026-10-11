struct S {
  int a;
  int b;
  int c;
};

int main() {
  S s = {.c = 3};
  S t = (S){.b = 2, .c = 3};
  return (s.a == 0 && s.b == 0 && s.c == 3 &&
          t.a == 0 && t.b == 2 && t.c == 3) ? 0 : 1;
}
