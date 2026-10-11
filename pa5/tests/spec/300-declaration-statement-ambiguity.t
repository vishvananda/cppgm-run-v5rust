// N3485 focus: 6.8 [stmt.ambig] declaration statement ambiguity resolution
int f() {
  int(a);
  a = 1;
  return a;
}

typedef int function;

void called(void *) {}

int g() {
  void (*function)(void *) = called;
  int spawned_thread;
  try {
    function(&spawned_thread);
  } catch (...) {
  }
  return 0;
}
