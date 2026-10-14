namespace Values { int value = 3; }
namespace Imported = Values;
int next() { return 7; }
int dead_plain() { return 0; thread_local int value = next(); }
int dead_constant() { return 0; thread_local int value = 4; }
int dead_static() { return 0; static thread_local int value = next(); }
int main() {
  namespace Local = Imported;
  using namespace Local;
  using Integer = int;
  enum Choice { first = 1, second = 2 };
  static_assert(sizeof(Integer) == 4, "integer size");
  Integer result = value + second;
  return result + dead_plain() + dead_constant() + dead_static();
}
