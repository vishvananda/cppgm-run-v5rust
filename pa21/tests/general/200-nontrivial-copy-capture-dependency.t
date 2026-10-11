struct A {
  int* count;
  A(int* p) : count(p) {}
  A(const A& a) : count(a.count) { ++*count; }
};
int main() { int n = 0; A a(&n); auto f = [a]() {}; return n; }
