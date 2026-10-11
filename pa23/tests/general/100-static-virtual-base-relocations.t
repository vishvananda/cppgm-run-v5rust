// A known complete object supplies its virtual-base layout.
struct Virtual { int value; virtual ~Virtual() {} };
struct Primary { int primary; virtual ~Primary() {} };
struct Base : Primary, virtual Virtual { int base; };
struct Object : Base { int object; };
Object object;
Object objects[2];
Virtual &reference = object;
Virtual *pointer = &object;
Virtual *explicit_pointer = static_cast<Virtual *>(&object);
Base &base_reference = object;
Virtual &alias_reference = base_reference;
Base *base_pointer = &object;
Virtual *alias_pointer = base_pointer;
Virtual &array_reference = objects[1];
int main() {
  reference.value = 7;
  array_reference.value = 11;
  Virtual *expected = static_cast<Virtual *>(&object);
  return object.value != 7 || pointer != expected || explicit_pointer != expected ||
    &alias_reference != expected || alias_pointer != expected ||
    objects[1].value != 11 || objects[0].value != 0 ||
    &array_reference != static_cast<Virtual *>(&objects[1]);
}
