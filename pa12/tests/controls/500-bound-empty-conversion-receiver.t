// expect exit 0
// N3485 8.5.3p5, 12.2p3: binding a conversion result to a base reference
// retains the receiver's full-expression lifetime and empty-object lowering.
int destroyed;
struct Base { int n; };
struct Derived : Base {
  Derived() { n = 7; }
};
Derived* target;
struct Receiver {
  operator Derived&() { return *target; }
  ~Receiver() { ++destroyed; }
};
int main() {
  Derived object;
  target = &object;
  const Base& first = true ? Receiver() : Receiver();
  if (destroyed != 1 || &first != &object) return 1;
  const Base& second = static_cast<const Base&>(Receiver());
  if (destroyed != 2 || &second != &object) return 2;
  return second.n != 7;
}
