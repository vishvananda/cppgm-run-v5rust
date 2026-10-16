// N3485 14.7.1p12: instantiate only omitted defaults, once per specialization;
// evaluate the cached default afresh on each runtime call; expect exit 0
int calls = 0;
int tick() { return ++calls; }
template<class T>
int use(T, int required = T::missing, int omitted = sizeof(T) + tick()) {
  return required + omitted;
}
template<class T>
int unused(T, int value = T::missing) { return value; }
int main() {
  if (use(1, 2) != 7) return 1;
  if (use(2, 2) != 8) return 2;
  if (use('x', 2) != 6) return 3;
  if (use(3, 2, 11) != 13 || calls != 3) return 4;
  return 0;
}
