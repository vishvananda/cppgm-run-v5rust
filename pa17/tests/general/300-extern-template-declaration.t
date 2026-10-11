template<class T>
void f(T);

extern template void f<int>(int);

int main() {
  return 0;
}
