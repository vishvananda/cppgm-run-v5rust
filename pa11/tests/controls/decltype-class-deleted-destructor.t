// expect reject
struct Object {
  Object() {}
  ~Object() = delete;
};
using Type = decltype((0, Object{}));
int main() { return 0; }
