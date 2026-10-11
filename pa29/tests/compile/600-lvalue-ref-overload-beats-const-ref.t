int pick(int &x) { return x + 1; }
int pick(const int &x) { return x + 2; }

int main() {
  int value = 4;
  return pick(value) == 5 ? 0 : 1;
}
