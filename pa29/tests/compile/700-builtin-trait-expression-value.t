struct Plain {
  int value;
};

int main() {
  bool trivial = __is_trivial(Plain);
  return trivial ? 0 : 1;
}
