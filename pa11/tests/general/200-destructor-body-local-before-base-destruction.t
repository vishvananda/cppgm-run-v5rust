int order;

struct Guard {
  ~Guard() { order = order * 10 + 1; }
};

struct Base {
  ~Base() { order = order * 10 + 2; }
};

struct Derived : Base {
  ~Derived() {
    Guard guard;
    order = order * 10 + 3;
  }
};

int main() {
  {
    Derived value;
  }
  return order == 312 ? 0 : 1;
}
