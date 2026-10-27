int live;
struct True { static const bool value = true; operator bool() const { return value; } };
struct False { static const bool value = false; operator bool() const { return value; } };
struct Object {
  Object() { ++live; }
  ~Object() { --live; }
  operator int() const { return 1; }
};
int left(bool value) { return value ? (True{} ? int(Object()) : int(Object())) : 3; }
int right(bool value) { return value ? 3 : (False{} ? int(Object()) : int(Object())); }
int either(bool value) {
  return value ? (True{} ? int(Object()) : int(Object()))
               : (False{} ? int(Object()) : int(Object()));
}
int main() {
  return left(true) + left(false) + right(true) + right(false)
       + either(true) + either(false) - 10 + live;
}
