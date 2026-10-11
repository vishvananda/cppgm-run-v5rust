template<class T>
struct Pair {
  T first;

  void copy(const Pair& p) {
    first = p.first;
  }
};

typedef Pair<int> pair_type;

int main() { return 0; }
