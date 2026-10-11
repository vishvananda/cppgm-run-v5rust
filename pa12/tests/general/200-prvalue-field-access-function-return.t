class P {
public:
  int first;
  int second;

  P() {
    first = 1;
    second = 7;
  }
};

P make() {
  P p;
  return p;
}

int main() {
  return make().second;
}
