// A static data-member initializer is not a complete-class context (9.2p2).
// The enclosing member body is not available for constant evaluation until
// the class completes; a constexpr initializer must reject this early call.
struct early {
  static constexpr int make() { return 7; }
  static constexpr int value = make();
};
int main() { return early::value; }
