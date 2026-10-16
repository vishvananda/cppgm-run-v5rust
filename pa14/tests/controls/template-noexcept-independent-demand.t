// expect exit 0
template<class T> int declared(T) noexcept(sizeof(T) == 4);
template<class T> int unused(T) noexcept(sizeof(T) == 4) { return T::missing; }
int main() {
  return noexcept(declared(1)) + !noexcept(declared('x'))
    + noexcept(unused(2)) - 3;
}
