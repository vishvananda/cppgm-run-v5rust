struct A { unsigned : 3; int x; int y; };
A make(int x) { return A{x}; }
int main() {
  A a = make(7);
  return a.x == 7 && a.y == 0 ? 0 : 1;
}
