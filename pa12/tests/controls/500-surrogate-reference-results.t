// expect exit 0
// N3485 13.3.1.1.2: function references and references to function
// pointers remain callable, without reading storage at a function's address.
typedef int Function(int);
typedef Function* Pointer;
int twice(int n) { return n * 2; }
Pointer pointer = twice;
struct Reference { operator Function&() { return twice; } };
struct RvalueReference { operator Function&&() { return static_cast<Function&&>(twice); } };
struct PointerReference { operator Pointer&() { return pointer; } };
struct PointerRvalueReference {
  operator Pointer&&() { return static_cast<Pointer&&>(pointer); }
};
int main() {
  Reference a;
  RvalueReference b;
  PointerReference c;
  PointerRvalueReference d;
  return a(3) != 6 || b(4) != 8 || c(5) != 10 || d(6) != 12;
}
