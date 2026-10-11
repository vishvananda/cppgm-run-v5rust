// Test null preservation before reading the virtual-base offset.
struct Left { int left; };
struct Right { int right; };
struct Virtual : Left, Right { virtual ~Virtual() {} };
struct Object : virtual Virtual { int value; };
Virtual *implicit_conversion(Object *pointer) { return pointer; }
Virtual *explicit_conversion(Object *pointer) { return static_cast<Virtual *>(pointer); }
Right *nested_conversion(Object *pointer) { return pointer; }
Virtual *reference_to_pointer(Object *&pointer) { return pointer; }
int main() {
  Object *pointer = nullptr;
  if (implicit_conversion(pointer) || explicit_conversion(pointer) ||
      nested_conversion(pointer) || reference_to_pointer(pointer)) return 1;
  Object object;
  pointer = &object;
  Virtual *expected = static_cast<Virtual *>(&object);
  return implicit_conversion(pointer) != expected ||
    explicit_conversion(pointer) != expected ||
    nested_conversion(pointer) != static_cast<Right *>(&object) ||
    reference_to_pointer(pointer) != expected;
}
