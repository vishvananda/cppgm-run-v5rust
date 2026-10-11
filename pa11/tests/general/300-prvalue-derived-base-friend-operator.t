struct Base {
  friend int operator-(const Base&, const Base&) { return 0; }
};

struct Derived : Base {};

int main() {
  return Derived() - Derived();
}
