// expect exit 0
// Both allocation branches and delete operands retain their temporary regions.
int destroyed;
struct Temporary { ~Temporary() { ++destroyed; } };
struct Object {
  Object(const Temporary&) {}
  ~Object() noexcept(false) {}
};
Object* make(bool branch) {
  return branch ? new Object(Temporary()) : new Object(Temporary());
}
Object* select(const Temporary&) { return new Object(Temporary()); }
void release(bool branch, Object* p) { delete (branch ? select(Temporary()) : p); }
void nested() { delete new Object(Temporary()); }
int main() {
  delete make(true);
  delete make(false);
  nested();
  if (destroyed != 3) return 1;
  release(true, 0);
  release(false, new Object(Temporary()));
  return destroyed == 6 ? 0 : 2;
}
