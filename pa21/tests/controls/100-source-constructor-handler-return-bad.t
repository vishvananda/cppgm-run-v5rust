// N3485 15.3p14: a constructor's function-try handler cannot return; expect reject
struct A {
  A() try {} catch (...) { return; }
};
A value;
int main() { return 0; }
