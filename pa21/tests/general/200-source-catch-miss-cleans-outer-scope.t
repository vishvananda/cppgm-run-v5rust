int destroyed = 0;

struct Guard {
  ~Guard() {
    destroyed = destroyed + 1;
  }
};

int main() {
  try {
    Guard guard;
    try {
      throw 7;
    } catch (long) {
      return 10;
    }
  } catch (int) {
    return destroyed == 1 ? 0 : 1;
  }
  return 2;
}
