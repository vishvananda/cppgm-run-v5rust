namespace N {

struct Iter {
  int *p;

  friend bool operator!=(const Iter &lhs, const Iter &rhs) {
    return lhs.p != rhs.p;
  }
};

}

int main() {
  int x = 0;
  int y = 0;
  N::Iter a = {&x};
  N::Iter b = {&y};
  return a != b ? 0 : 1;
}
