static __inline__ int add(int a, int b) {
  return a + b;
}

__thread int tls;

int main() {
  tls = add(1, 2);
  return tls - 3;
}
