struct Counter {
  int current;
  int step;

  int operator++(int) {
    int old = current;
    current = current + step;
    return old;
  }
};

int main() {
  Counter c;
  c.current = 7;
  c.step = 3;
  int old = c++;
  return old == 7 && c.current == 10 && c.step == 3 ? 0 : 1;
}
