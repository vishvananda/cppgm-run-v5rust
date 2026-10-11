struct Base {
  int x;
  Base(int v) : x(v) {}
};

struct Derived : Base {
  int y;
  Derived(int a, int b) : Base(a), y(b) {}
};

int main() {
  Derived d(4, 5);
  return d.x + d.y - 9;
}
