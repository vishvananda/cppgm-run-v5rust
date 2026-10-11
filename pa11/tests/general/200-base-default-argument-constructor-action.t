struct Base {
  int x;
  Base(int v = 4) : x(v) {}
};

struct Derived : Base {
  int y;
  Derived() : y(5) {}
};

int main() {
  Derived d;
  return d.x + d.y - 9;
}
