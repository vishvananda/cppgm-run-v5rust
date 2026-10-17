// N3485 5.2.2, 9.3.1: invoking a reference member's operator() preserves
// the implicit object's field designation, including a qualified base; expect exit 0
struct FunctionBase {
  int operator()(int value) const { return value + 2; }
};
struct Function : FunctionBase {};
struct Storage { Function const& function; };
struct Holder : Storage {
  Holder(Function const& f) : Storage{f} {}
  int run(int value) const { return ((Storage::function))(value); }
};
int main() {
  Function function;
  Holder holder(function);
  return holder.run(3) == 5 ? 0 : 1;
}
