// expect reject
// N3485 11.4: a derived friend cannot call a protected base operator on Base.
struct Base {
protected:
  int operator()(int n) { return n; }
};
struct Derived : Base {
  friend int evaluate(Base &base) { return base(3); }
};
