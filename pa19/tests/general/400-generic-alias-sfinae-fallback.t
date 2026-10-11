template<class T>
using member_type_t = typename T::type;

template<class T>
int pick(T, member_type_t<T>* = 0) {
  return 1;
}

int pick(...) {
  return 2;
}

int main() {
  return pick(0);
}
