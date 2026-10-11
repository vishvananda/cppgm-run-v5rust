enum E { A, B };

struct Pair {
  const char *prefix;
  E kind;
};

int main() {
  const Pair arr[] = {{"x", A}, {"y", B}};
  return arr[0].kind == A && arr[1].kind == B ? 0 : 1;
}
