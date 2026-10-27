// A copy constructor may have trailing defaulted parameters; explicit
// defaulting rejects their default arguments after classifying the copy kind.
struct Value {
  int member;
  Value(const Value&, int unused = 9) = default;
};
