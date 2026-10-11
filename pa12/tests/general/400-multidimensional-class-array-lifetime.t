int live;
struct X { X() { ++live; } ~X() { --live; } };
int main() {
  X (*p)[2] = new X[2][2];
  int count = live;
  delete[] p;
  return count != 4 || live;
}
