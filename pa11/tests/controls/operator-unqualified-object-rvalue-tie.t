// expect reject
struct Object {
  Object() {}
  int operator+(int) { return 1; }
  friend int operator+(Object&&, int) { return 2; }
};
int main() { return Object() + 3; }
