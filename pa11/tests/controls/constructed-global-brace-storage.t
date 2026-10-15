// expect exit 0
struct Value {
  int n;
  Value(int v) : n(v) {}
};
Value global{3};
namespace { Value local{4}; }
int main() { return global.n != 3 || local.n != 4; }
