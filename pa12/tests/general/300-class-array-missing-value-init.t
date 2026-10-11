struct C {
  int x;
  C() : x(7) {}
};

int main() {
  C arr[2] = { C() };
  return arr[1].x == 7 ? 0 : 1;
}
