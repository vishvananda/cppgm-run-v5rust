// N3485 3.5p4,p8, 14.4p1: address arguments inherit entity locality; expect exit 0
namespace {
struct Local {
  static int member() { return 3; }
};
struct Argument {};
}
extern "C" int c_function(Argument*) { return 5; }
extern "C" Argument c_object;
Argument c_object;
template<int (*P)()> int member_use() { return P(); }
template<int (*P)(Argument*)> int c_use() { return P(nullptr); }
template<Argument* P> int object_use() { return P == &c_object; }
int main() {
  return member_use<&Local::member>() + c_use<&c_function>()
      + object_use<&c_object>() - 9;
}
