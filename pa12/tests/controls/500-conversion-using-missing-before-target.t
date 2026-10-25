// expect reject
// Missing conversion lookup must fail before analyzing the target type.
struct Empty {};
struct Derived : Empty {
  using Empty::operator unknown;
};
