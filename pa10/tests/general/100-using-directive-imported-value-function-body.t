namespace N {
  const int K = 3;
}

using namespace N;

int g() {
  return K + 1;
}

int main() {
  return g() - 4;
}
