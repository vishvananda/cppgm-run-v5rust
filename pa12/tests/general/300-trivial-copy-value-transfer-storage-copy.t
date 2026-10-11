struct PairIter {
  int* first;
  int* second;
  long long tag;
};

PairIter forward(PairIter it) {
  return it;
}

int main() {
  int x = 0;
  PairIter a = {&x, &x, 7};
  PairIter b = forward(a);
  return b.first == &x && b.second == &x && b.tag == 7 ? 0 : 1;
}
