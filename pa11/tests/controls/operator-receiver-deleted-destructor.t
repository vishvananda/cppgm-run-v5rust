// expect reject
struct Object {
  Object() {}
  ~Object() = delete;
  int operator+(int) { return 1; }
};
int main() { return Object() + 3; }
