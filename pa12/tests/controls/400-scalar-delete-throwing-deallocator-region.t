// expect exit 0
// A transient deallocator call closes its region and emits the local cleanup.
int destroyed;
int freed;
struct Local { ~Local() { ++destroyed; } };
struct Object {
  static void operator delete(void* p) noexcept(false) {
    ++freed;
    ::operator delete(p);
  }
};
void release(Object* p) { Local local; delete p; }
int main() {
  release(new Object);
  return destroyed == 1 && freed == 1 ? 0 : 1;
}
