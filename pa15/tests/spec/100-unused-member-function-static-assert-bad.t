// VALIDATION: compile-fail
// N3485 focus: 7 [dcl.dcl] static_assert-declaration

struct YS {
  int x;
  char c;

  int f() {
    static_assert(sizeof(x) == sizeof(c), "bad");
    return 0;
  }
};

int main() {
  return 0;
}
