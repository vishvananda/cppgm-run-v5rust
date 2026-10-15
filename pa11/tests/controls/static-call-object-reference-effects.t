// expect exit 0
struct Object {
  static int method(int value) { return value; }
};
int effects;
Object& object(Object* p) { ++effects; return *p; }
int argument() { ++effects; return effects; }
int call(Object* p) { return object(p).method(argument()); }
int main() { return 0; }
