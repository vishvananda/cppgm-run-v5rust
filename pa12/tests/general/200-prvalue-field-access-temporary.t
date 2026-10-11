class P {
public:
  int first;
  int second;

  P() {
    first = 1;
    second = 9;
  }
};

int main() {
  return P().second;
}
