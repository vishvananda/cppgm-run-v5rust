template<class T>
T f(T x) {
  __asm__ __volatile__("pause");
  return x;
}

int main() {
  return 0;
}
