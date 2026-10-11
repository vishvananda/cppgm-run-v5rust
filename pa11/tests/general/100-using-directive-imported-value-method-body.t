namespace N {
  const int K = 3;
}

using namespace N;

struct X {
  int g() {
    return K + 1;
  }
};

int main() {
  X x;
  return x.g() - 4;
}
