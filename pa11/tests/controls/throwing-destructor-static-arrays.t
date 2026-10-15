// N3485 3.6.2, 3.6.3p1, 15.2p2: startup array construction cleanup;
// expect exit 0
int alive;
struct Object {
  Object() { ++alive; }
  ~Object() noexcept(false) { --alive; }
};
Object first[2], second[2], large[9];
thread_local Object per_thread[9];
int main() {
  if (alive != 13) return 1;
  Object* objects = per_thread;
  return !objects || alive != 22;
}
