template<class T>
T add(T lhs, T rhs = T(4)) {
  return lhs + rhs;
}

int main() {
  return add<int>(3);
}
