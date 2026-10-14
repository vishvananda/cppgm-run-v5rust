// expect reject
// N3485 11.4: a derived member cannot use a protected operator on a base object.
struct Base {
protected:
  int operator+(int n) { return n; }
};
struct Derived : Base {
  int evaluate(Base &base) { return base + 3; }
};
