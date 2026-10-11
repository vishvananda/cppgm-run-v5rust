// Static addresses do not require constant object values.
struct Left { int left; };
struct Right { int right; };
struct Object : Left, Right { int value; };
Object object;
Right &reference = object;
Right *pointer = &object;
Right *explicit_pointer = static_cast<Right *>(&object);
Right &explicit_reference = static_cast<Right &>(object);
Right const &constant_reference = object;
Object *inverse_pointer = static_cast<Object *>(pointer);
Right *null_pointer = static_cast<Right *>(static_cast<Object *>(nullptr));
Right &local_reference() { static Right &result = object; return result; }
Right *local_pointer() { static Right *result = &object; return result; }
int main() {
  reference.right = 7;
  Right *expected = static_cast<Right *>(&object);
  return object.right != 7 || pointer != expected || explicit_pointer != expected ||
    &explicit_reference != expected || &constant_reference != expected ||
    inverse_pointer != &object || null_pointer != nullptr ||
    &local_reference() != expected || local_pointer() != expected;
}
