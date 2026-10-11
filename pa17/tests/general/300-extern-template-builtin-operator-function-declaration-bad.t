template<class T>
int operator+(T, T);

extern template int operator+<int>(int, int);

int main() {
  return 0;
}
