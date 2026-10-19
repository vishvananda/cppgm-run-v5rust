// Frozen global_lowering.cpp::static_lvalue binds reference-form casts as
// relocations, including later reference and pointer initializers; expect exit 0
struct B { virtual ~B() {} };
struct D : B { int value; };
D object;
B& base = object;
D& cast = dynamic_cast<D&>(base);
D& alias = cast;
D* pointer = &dynamic_cast<D&>(base);
char* bytes = reinterpret_cast<char*>(&dynamic_cast<D&>(base)) + 1;
int& member = dynamic_cast<D&>(base).value;
int main() {
  member = 7;
  return &cast != &object || &alias != &object || pointer != &object || object.value != 7
    || bytes != reinterpret_cast<char*>(&object) + 1;
}
