// expect exit 0
// An anonymous default-argument body has no caller object. A nested real member
// body keeps its own object and the outer default's closure naming context.
template<class T> struct Owner {
  static int read(int n = []() {
    struct Local { int n = 7; int read() { return this->n; } };
    return Local{}.read();
  }()) { return n; }
};
int main() { return Owner<int>::read() - 7; }
