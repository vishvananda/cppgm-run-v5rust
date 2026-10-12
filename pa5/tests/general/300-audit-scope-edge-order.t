namespace First { int word; }
namespace Second { typedef int word; }
using namespace First;
using namespace Second;
void imported() { word * p; }

struct ValueBase { int word; };
struct TypeBase { typedef int word; };
struct TypeLast : ValueBase, TypeBase {
  void f() { word * p; }
};
struct ValueLast : TypeBase, ValueBase {
  void f() { word * p; }
};
