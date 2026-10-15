// N3485 6.6p2, 15.2p1, 15.5.1p1: potentially throwing destructors;
// expect exit 0
int alive;
int destroyed;
struct Object {
  Object() { ++alive; }
  ~Object() noexcept(false) { --alive; ++destroyed; }
};
int shared_return(int choice) {
  Object first, second, third;
  if (choice == 1) return 3;
  if (choice == 2) return 4;
  return 5;
}
void transfers() {
  for (int i = 0; i != 3; ++i) {
    Object first, second;
    if (i == 0) continue;
    if (i == 1) break;
  }
  int count = 0;
again:
  Object first, second;
  if (++count == 2) return;
  goto again;
}
int main() {
  if (shared_return(1) != 3 || shared_return(2) != 4 ||
      shared_return(0) != 5 || alive != 0) return 1;
  transfers();
  if (alive != 0 || destroyed != 17) return 2;
  {
    Object first;
    Object array[2][3];
    if (alive != 7) return 3;
  }
  return alive != 0 || destroyed != 24;
}
