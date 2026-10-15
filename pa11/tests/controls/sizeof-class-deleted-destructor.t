// expect reject
struct Object {
  Object() {}
  ~Object() = delete;
};
int main() { return sizeof((Object())); }
