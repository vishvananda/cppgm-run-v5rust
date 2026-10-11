namespace std { template<typename T> class initializer_list; }

int sum(std::initializer_list<int> xs) {
  int s = 0;
  for (auto x : xs) s = s + x;
  return s;
}

int main() {
  return sum({1, 2, 3});
}
