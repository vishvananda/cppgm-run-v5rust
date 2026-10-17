// N3485 7.1.3p8, 14.5.6.1p5: a typedef-name names the class whose member
// template is redeclared, including dependent head and return types.
// expect exit 0
struct Owner {
  template<class U, decltype(sizeof(U)) N> int count();
  template<class U> decltype(sizeof(U)) width(U);
};
typedef Owner Alias;
using Other = Alias;
template<class V, decltype(sizeof(V)) M> int Other::count() { return M; }
template<class V> decltype(sizeof(V)) Alias::width(V) { return sizeof(V); }

struct Distinct {
  template<class U, decltype(sizeof(U)) N> int count();
};
using Separate = Distinct;
template<class V, decltype(sizeof(V)) M> int Separate::count() { return M + 1; }

int main() {
  Owner owner;
  Distinct distinct;
  return owner.count<char, 3>() + owner.width(0L) + distinct.count<int, 4>() - 16;
}
