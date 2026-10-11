struct Iter {
  int value;

  Iter& operator*() {
    return *this;
  }
};

int main() {
  Iter a;
  Iter b;
  a.value = 7;
  b.value = 1;
  b = a;
  return (*b).value == 7 ? 0 : 1;
}
