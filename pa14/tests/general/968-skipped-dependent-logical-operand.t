template<class T> int positive(char (*)[true || sizeof(T)]) { return 1; }
template<class T> int negative(char (*)[1 + (false && sizeof(T))]) { return 2; }
int ordinary(char (*)[true || (1 / 0)]) { return 3; }
int main() {
  return positive<int>(nullptr) + negative<int>(nullptr) + ordinary(nullptr) - 6;
}
