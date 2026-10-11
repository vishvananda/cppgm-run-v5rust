// N3485 6.8: declarations win when both statement parses are possible.
struct C {
  int m;
  C(int = 0, int = 0);
  C* operator->();
  C& operator++(int);
};
int operator<<(C, int);
void expressions(int a, int c) {
  C(a)->m = 7;
  C(a)++;
  C(a,5)<<c;
}
void declarations() {
  C(*d)(int);
  C(e)[5];
  C(f) = {1,2};
  C(*g)(nullptr);
  C(a);
  C(*b)();
  C(c)=7;
}
void comma_declarations(int h) {
  C(d), e, f=3;
  C(g)(h,2);
}
