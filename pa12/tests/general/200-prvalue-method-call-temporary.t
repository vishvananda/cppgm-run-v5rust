class P {
public:
  int second;

  P() {
    second = 4;
  }

  int get() {
    return second;
  }
};

int main() {
  return P().get();
}
