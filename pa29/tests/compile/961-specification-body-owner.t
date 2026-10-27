void ordinary() noexcept(__builtin_FUNCTION()[0]==0) {}
struct Example {
  void member() const noexcept(sizeof(this)==8 && __builtin_FUNCTION()[0]==0) {}
};
int main() {
  void local() noexcept(__builtin_FUNCTION()[0]==0);
  Example object;
  static_assert(noexcept(ordinary()), "ordinary specification has no body owner");
  static_assert(noexcept(object.member()), "specification keeps its implicit object");
  static_assert(noexcept(local()), "local declaration has no body owner");
  ordinary();
  object.member();
  return 0;
}
