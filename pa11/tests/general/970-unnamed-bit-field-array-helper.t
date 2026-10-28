struct A { unsigned : 3; int x; };
int sum(int x) {
  A a[2] = {{x}, {x + 1}};
  return a[0].x + a[1].x;
}
int main() { return sum(7) == 15 ? 0 : 1; }
