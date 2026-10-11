int invoke(int (^cb)(int), int x) {
  return cb(x);
}

int main() {
  return 0;
}
