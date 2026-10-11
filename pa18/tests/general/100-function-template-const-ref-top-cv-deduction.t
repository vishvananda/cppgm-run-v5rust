template<class T>
const T& max_like(const T& a, const T& b) {
  return a < b ? b : a;
}

unsigned long f(unsigned long current) {
  return max_like(current, 2 * current);
}

int main() {
  return f(3) == 6 ? 0 : 1;
}
